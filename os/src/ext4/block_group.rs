use alloc::sync::Arc;

use crate::{
    drivers::block::{block_cache::get_block_cache, block_dev::BlockDevice},
    mutex::SpinNoIrqLock,
};

use super::block_op::Ext4Bitmap;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Ext4GroupDescDisk {
    block_bitmap_lo: u32,      // block位图的起始块号(低32位)
    inode_bitmap_lo: u32,      // inode位图的起始块号(低32位)
    inode_table_lo: u32,       // inode表的起始块号(低32位)
    free_blocks_count_lo: u16, // 空闲的block总数(低16位)
    free_inodes_count_lo: u16, // 空闲的inode总数(低16位)
    used_dirs_count_lo: u16,   // 使用的目录总数(低16位)
    pub flags: u16,            // 块组标志, EXT$_BG_flags(INODE_UNINIT, etc)
    exclude_bitmap_lo: u32,    // 快照排除位图
    block_bitmap_csum_lo: u16, // block位图校验和(低16位, crc32c(s_uuid+grp_num+bitmap)) LE
    inode_bitmap_csum_lo: u16, // inode位图校验和(低16位, crc32c(s_uuid+grp_num+bitmap)) LE
    itable_unused_lo: u16,     // 未使用的inode 数量(低16位)
    checksum: u16,             // crc16(sb_uuid+group_num+desc)
    block_bitmap_hi: u32,      // block位图的起始块号(高32位)
    inode_bitmap_hi: u32,      // inode位图的起始块号(高32位)
    inode_table_hi: u32,       // inode表的起始块号(高32位)
    free_blocks_count_hi: u16, // 空闲的block总数(高16位)
    free_inodes_count_hi: u16, // 空闲的inode总数(高16位)
    used_dirs_count_hi: u16,   // 使用的目录总数(高16位)
    itable_unused_hi: u16,     // (已分配但未被初始化)未使用的inode 数量(高16位)
    exclude_bitmap_hi: u32,    // 快照排除位图
    block_bitmap_csum_hi: u16, // crc32c(s_uuid+grp_num+bitmap)的高16位
    inode_bitmap_csum_hi: u16, // crc32c(s_uuid+grp_num+bitmap)的高16位
    reserved: u32,             // 保留字段, 填充
}

impl Ext4GroupDescDisk {
    pub fn is_inode_uninit(&self) -> bool {
        self.flags & 0x1 == 0x1
    }
    pub fn inode_table(&self) -> u64 {
        (self.inode_table_hi as u64) << 32 | self.inode_table_lo as u64
    }
    pub fn block_bitmap(&self) -> u64 {
        (self.block_bitmap_hi as u64) << 32 | self.block_bitmap_lo as u64
    }
    pub fn inode_bitmap(&self) -> u64 {
        (self.inode_bitmap_hi as u64) << 32 | self.inode_bitmap_lo as u64
    }
    pub fn exclude_bitmap(&self) -> u64 {
        (self.exclude_bitmap_hi as u64) << 32 | self.exclude_bitmap_lo as u64
    }
    pub fn free_blocks_count(&self) -> u32 {
        (self.free_blocks_count_hi as u32) << 16 | self.free_blocks_count_lo as u32
    }
    pub fn free_inodes_count(&self) -> u32 {
        (self.free_inodes_count_hi as u32) << 16 | self.free_inodes_count_lo as u32
    }
    pub fn used_dirs_count(&self) -> u32 {
        (self.used_dirs_count_hi as u32) << 16 | self.used_dirs_count_lo as u32
    }
    pub fn itable_unused(&self) -> u32 {
        (self.itable_unused_hi as u32) << 16 | self.itable_unused_lo as u32
    }
}

pub struct GroupDesc {
    pub inode_table: u64,
    pub block_bitmap: u64,
    pub inode_bitmap: u64,
    pub exclude_bitmap: u64,

    inner: SpinNoIrqLock<GroupDescInner>,
}

impl GroupDesc {
    pub fn inode_table(&self) -> u64 {
        self.inode_table
    }
}

impl GroupDesc {
    pub fn new(group_desc_disk: &Ext4GroupDescDisk) -> Self {
        Self {
            inode_table: group_desc_disk.inode_table(),
            block_bitmap: group_desc_disk.block_bitmap(),
            inode_bitmap: group_desc_disk.inode_bitmap(),
            exclude_bitmap: (group_desc_disk.exclude_bitmap_hi as u64) << 32
                | group_desc_disk.exclude_bitmap_lo as u64,
            inner: SpinNoIrqLock::new(GroupDescInner::new(
                group_desc_disk.free_blocks_count(),
                group_desc_disk.free_inodes_count(),
                group_desc_disk.used_dirs_count(),
                group_desc_disk.itable_unused(),
            )),
        }
    }
    /// 在块组的inode_bitmap中分配一个inode
    /// 注意这个inode_num是相对于块组的inode_table的inode_num
    /// 调用者需要将inode_num转换为全局的inode_num(加上inodes_per_group * group_num)
    pub fn alloc_inode(
        &self,
        block_device: Arc<dyn BlockDevice>,
        ext4_block_size: usize,
        is_dir: bool,
    ) -> Option<usize> {
        let mut inner = self.inner.lock();
        // 检查当前块组是否还有空闲的inode
        if inner.free_inodes_count > 0 {
            // 设置inode位图中的inode为已分配
            // 修改bg的used_dirs_count, free_inodes_count, checksum, unused_inodes_count
            let inode_num = Ext4Bitmap::new(
                get_block_cache(self.inode_bitmap as usize, block_device, ext4_block_size)
                    .lock()
                    .get_mut(0),
            )
            .alloc()?;
            inner.free_inodes_count -= 1;
            // Todo: 设置块组的checksum
            if is_dir {
                inner.used_dirs_count += 1;
            }
            return Some(inode_num);
        }
        return None;
    }
}

pub struct GroupDescInner {
    free_blocks_count: u32,
    free_inodes_count: u32,
    used_dirs_count: u32,
    itable_unused: u32,
}

impl GroupDescInner {
    pub fn new(
        free_blocks_count: u32,
        free_inodes_count: u32,
        used_dirs_count: u32,
        itable_unused: u32,
    ) -> Self {
        Self {
            free_blocks_count,
            free_inodes_count,
            used_dirs_count,
            itable_unused,
        }
    }
}
