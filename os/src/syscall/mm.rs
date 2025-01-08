use crate::{
    config::{MMAP_MIN_ADDR, PAGE_SIZE, PAGE_SIZE_BITS},
    mm::{MapPermission, VPNRange, VirtAddr, VirtPageNum},
    task::current_task,
    utils::{ceil_to_page_size, floor_to_page_size},
};
use bitflags::bitflags;

pub fn sys_brk(brk: usize) -> isize {
    log::info!("sys_brk: brk: {:#x}", brk);
    let current_task = current_task();
    // sbrk(0)是获取当前program brk(堆顶)
    if brk == 0 {
        return current_task.inner.lock().memory_set.brk as isize;
    }
    let current_mm = &mut current_task.inner.lock().memory_set;
    let current_brk = current_mm.brk;
    let heap_bottom = current_mm.heap_bottom;
    // (start_vpn, end_vpn)是需要增/删的区间
    let start_vpn = VirtPageNum::from(ceil_to_page_size(current_brk) >> PAGE_SIZE_BITS);
    let new_end_vpn = VirtPageNum::from(ceil_to_page_size(brk) >> PAGE_SIZE_BITS);
    if brk < heap_bottom {
        // brk小于堆底, 不合法
        log::error!("[sys_brk] brk {:#x} < heap_bottom {:#x}", brk, heap_bottom);
        return -1;
    } else if brk > ceil_to_page_size(current_brk) {
        // 需要分配页
        if current_brk == heap_bottom {
            // 初始分配堆空间
            log::info!(
                "[sys_brk] init heap space: {:#x} - {:#x}",
                heap_bottom,
                new_end_vpn.0 << PAGE_SIZE_BITS
            );
            current_mm.insert_framed_area_va(
                VirtAddr::from(heap_bottom),
                VirtAddr::from(new_end_vpn.0 << PAGE_SIZE_BITS),
                MapPermission::R | MapPermission::W | MapPermission::U,
            );
        } else {
            current_mm.remap_area_with_start_vpn(start_vpn, new_end_vpn);
        }
    } else if brk < floor_to_page_size(current_brk) {
        // 需要释放页, 若start_vpn == new_end_vpn, 会将空间删除
        current_mm.remap_area_with_start_vpn(start_vpn, new_end_vpn);
    } else {
        // brk在同一页, 不用alloc/dealloc页
        // 页内偏移
    }
    current_mm.brk = brk;
    0
}

bitflags! {
    /// MMAP memeory protection
    pub struct MMAPPROT: u32 {
        /// Readable
        const PROT_READ = 1 << 0;
        /// Writeable
        const PROT_WRITE = 1 << 1;
        /// Executable
        const PROT_EXEC = 1 << 2;
    }
}

impl From<MMAPPROT> for MapPermission {
    fn from(prot: MMAPPROT) -> Self {
        let mut map_permission = MapPermission::from_bits(0).unwrap();
        if prot.contains(MMAPPROT::PROT_READ) {
            map_permission |= MapPermission::R;
        }
        if prot.contains(MMAPPROT::PROT_WRITE) {
            map_permission |= MapPermission::W;
        }
        if prot.contains(MMAPPROT::PROT_EXEC) {
            map_permission |= MapPermission::X;
        }
        map_permission
    }
}

bitflags! {
    /// determines whether updates to the mapping are visible to other processes mapping the same region, and whether
    /// updates are carried through to the underlying file.
    pub struct MMAPFLAGS: u32 {
        /// MAP_SHARED
        const MAP_SHARED = 1 << 0;
        /// MAP_PRIVATE
        const MAP_PRIVATE = 1 << 1;
        /// 以上两种只能选一
        /// MAP_FIXED, 一定要映射到addr, 不是作为hint, 要取消原来位置的映射
        const MAP_FIXED = 1 << 4;
        /// MAP_ANONYMOUS, 需要fd为-1, offset为0
        const MAP_ANONYMOUS = 1 << 5;
    }
}

// Todo: 别用unwarp
pub fn sys_mmap(
    _hint: usize,
    len: usize,
    prot: usize,
    flags: usize,
    fd: i32,
    offset: usize,
) -> isize {
    log::error!(
        "sys_mmap: hint: {:#x}, len: {:#x}, prot: {:#x}, flags: {:#x}, fd: {:#x}, offset: {:#x}",
        _hint,
        len,
        prot,
        flags,
        fd,
        offset
    );
    //处理参数
    let prot = MMAPPROT::from_bits(prot as u32).unwrap();
    let flags = MMAPFLAGS::from_bits(flags as u32).unwrap();
    let task = current_task();
    if len == 0 {
        return -22;
    }
    // mmap区域最低地址为MMAP_MIN_ADDR
    let mut permission: MapPermission = prot.into();
    // 注意加上U权限
    permission |= MapPermission::U;
    // 匿名映射
    if flags.contains(MMAPFLAGS::MAP_ANONYMOUS) {
        //需要fd为-1, offset为0
        if fd != -1 || offset != 0 {
            return -22;
        }
        let mut inner = task.inner.lock();
        // start可以保证是页对齐的
        let start = inner.memory_set.mmap_start;
        let vpn_range = inner.memory_set.get_unmapped_area(start, len);
        inner
            .memory_set
            .insert_framed_area_vpn_range(vpn_range, permission);
        return start as isize;
    } else {
        // 文件映射
        // Todo:fake
        // 需要offset为page aligned
        if offset % PAGE_SIZE != 0 {
            return -22;
        }
        // 读取文件
        let file = task
            .inner_handler(|inner| inner.fd_table[fd as usize].clone())
            .unwrap();
        let mut inner = task.inner.lock();
        let start = inner.memory_set.mmap_start;
        let vpn_range = inner.memory_set.get_unmapped_area(start, len);
        //task.inner_handler(|inner| inner.memory_set.page_table.dump_all());
        inner
            .memory_set
            .insert_framed_area_vpn_range(vpn_range, permission);
        let buf = unsafe { core::slice::from_raw_parts_mut(start as *mut u8, len) };
        let origin_offset = file.get_meta().offset;
        file.seek(offset);
        file.read(buf);
        file.seek(origin_offset);
        log::error!("mmap start: {:#x}", start as isize);
        log::error!("mmap content: {:?}", buf);
        return start as isize;
    }
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    // start必须页对齐, 且要大于等于MMAP_MIN_ADDR
    if start % PAGE_SIZE != 0 || len == 0 || start < MMAP_MIN_ADDR {
        return -22;
    }
    let start_vpn = VirtPageNum::from(start >> PAGE_SIZE_BITS);
    let end_vpn = VirtPageNum::from(ceil_to_page_size(start + len) >> PAGE_SIZE_BITS);
    log::error!(
        "sys_munmap: start: {:#x}, len: {:#x}, end: {:#x}, start_vpn: {:#x}, end_vpn: {:#x}",
        start,
        len,
        start + len,
        start_vpn.0,
        end_vpn.0
    );
    let unmap_vpn_range = VPNRange::new(start_vpn, end_vpn);
    current_task()
        .inner_handler(|inner| inner.memory_set.remove_area_with_overlap(unmap_vpn_range));
    0
}