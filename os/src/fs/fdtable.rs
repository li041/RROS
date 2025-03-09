use core::sync::atomic::AtomicUsize;

use crate::mutex::SpinNoIrqLock;

use super::{file::FileOp, Stdin, Stdout};
use alloc::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    sync::Arc,
};

// 进程的文件描述符表
pub struct FdTable {
    table: SpinNoIrqLock<BTreeMap<usize, Arc<dyn FileOp>>>,
    free_fds: SpinNoIrqLock<BTreeSet<usize>>,
    next_fd: AtomicUsize,
}

impl FdTable {
    // 创建一个新的FdTable, 并初始化0(Stdin), 1(Stdout), 2(Stderr)三个文件描述符
    // Todo: stderr, 现在暂时使用stdout
<<<<<<< HEAD
<<<<<<< HEAD
    pub fn new() -> Self {
=======
    pub fn new() -> Arc<Self> {
>>>>>>> 8162fada35bdfa8533bc38451ef1b322d3374e58
=======
    pub fn new() -> Arc<Self> {
>>>>>>> 13a1da6b38b24cc7e0cb99a67db59f853588ac62
        let mut fd_table: BTreeMap<usize, Arc<dyn FileOp>> = BTreeMap::new();
        fd_table.insert(0, Arc::new(Stdin));
        fd_table.insert(1, Arc::new(Stdout));
        // Todo: stderr, 现在暂时使用stdout
        fd_table.insert(2, Arc::new(Stdout));
<<<<<<< HEAD
<<<<<<< HEAD
        Self {
            table: SpinNoIrqLock::new(fd_table),
            free_fds: SpinNoIrqLock::new(BTreeSet::new()),
            next_fd: AtomicUsize::new(3), // 0, 1, 2 are reserved for stdin, stdout, stderr
        }
    }
    // 从已有的FdTable中创建一个新的FdTable, 复制已有的文件描述符
    pub fn from_existed_user(parent_table: &FdTable) -> Self {
=======
=======
>>>>>>> 13a1da6b38b24cc7e0cb99a67db59f853588ac62
        Arc::new(Self {
            table: SpinNoIrqLock::new(fd_table),
            free_fds: SpinNoIrqLock::new(BTreeSet::new()),
            next_fd: AtomicUsize::new(3), // 0, 1, 2 are reserved for stdin, stdout, stderr
        })
    }
    pub fn reset(&self) {
        let mut fd_table: BTreeMap<usize, Arc<dyn FileOp>> = BTreeMap::new();
        fd_table.insert(0, Arc::new(Stdin));
        fd_table.insert(1, Arc::new(Stdout));
        // Todo: stderr, 现在暂时使用stdout
        fd_table.insert(2, Arc::new(Stdout));
        *self.table.lock() = fd_table;
        self.free_fds.lock().clear();
        self.next_fd.store(3, core::sync::atomic::Ordering::SeqCst);
    }
    // 从已有的FdTable中创建一个新的FdTable, 复制已有的文件描述符
    pub fn from_existed_user(parent_table: &FdTable) -> Arc<Self> {
<<<<<<< HEAD
>>>>>>> 8162fada35bdfa8533bc38451ef1b322d3374e58
=======
>>>>>>> 13a1da6b38b24cc7e0cb99a67db59f853588ac62
        let mut fd_table: BTreeMap<usize, Arc<dyn FileOp>> = BTreeMap::new();
        for (fd, file) in parent_table.table.lock().iter() {
            fd_table.insert(*fd, file.clone());
        }
<<<<<<< HEAD
<<<<<<< HEAD
        Self {
=======
        Arc::new(Self {
>>>>>>> 8162fada35bdfa8533bc38451ef1b322d3374e58
=======
        Arc::new(Self {
>>>>>>> 13a1da6b38b24cc7e0cb99a67db59f853588ac62
            table: SpinNoIrqLock::new(fd_table),
            free_fds: SpinNoIrqLock::new(BTreeSet::new()),
            next_fd: AtomicUsize::new(
                parent_table
                    .next_fd
                    .load(core::sync::atomic::Ordering::SeqCst),
            ),
<<<<<<< HEAD
<<<<<<< HEAD
        }
    }
    /// 分配文件描述符, 将文件插入FdTable中
    pub fn alloc_fd(&mut self, file: Arc<dyn FileOp + Send + Sync>) -> usize {
=======
=======
>>>>>>> 13a1da6b38b24cc7e0cb99a67db59f853588ac62
        })
    }
    /// 分配文件描述符, 将文件插入FdTable中
    pub fn alloc_fd(&self, file: Arc<dyn FileOp + Send + Sync>) -> usize {
<<<<<<< HEAD
>>>>>>> 8162fada35bdfa8533bc38451ef1b322d3374e58
=======
>>>>>>> 13a1da6b38b24cc7e0cb99a67db59f853588ac62
        let mut free_fds = self.free_fds.lock();
        // 优先使用free_fds中的最小的fd
        let fd = if let Some(&fd) = free_fds.iter().next() {
            free_fds.remove(&fd);
            fd
        } else {
            self.next_fd
                .fetch_add(1, core::sync::atomic::Ordering::SeqCst)
        };
        self.table.lock().insert(fd, file);
        fd
    }
    // 通过fd获取文件
    pub fn get_file(&self, fd: usize) -> Option<Arc<dyn FileOp>> {
        self.table.lock().get(&fd).cloned()
    }
<<<<<<< HEAD
<<<<<<< HEAD
    pub fn close(&mut self, fd: usize) -> bool {
=======
    pub fn close(&self, fd: usize) -> bool {
>>>>>>> 8162fada35bdfa8533bc38451ef1b322d3374e58
=======
    pub fn close(&self, fd: usize) -> bool {
>>>>>>> 13a1da6b38b24cc7e0cb99a67db59f853588ac62
        if self.table.lock().remove(&fd).is_some() {
            self.free_fds.lock().insert(fd);
            true
        } else {
            log::error!("[FdTable::close] fd not found: {}", fd);
            false
        }
    }
    // 清空FdTable
<<<<<<< HEAD
<<<<<<< HEAD
    pub fn clear(&mut self) {
=======
    pub fn clear(&self) {
>>>>>>> 8162fada35bdfa8533bc38451ef1b322d3374e58
=======
    pub fn clear(&self) {
>>>>>>> 13a1da6b38b24cc7e0cb99a67db59f853588ac62
        self.table.lock().clear();
        self.free_fds.lock().clear();
        self.next_fd.store(3, core::sync::atomic::Ordering::SeqCst);
    }
    // 给dup2使用, 将new_fd(并不是进程所能分配的最小描述符)指向old_fd的文件
<<<<<<< HEAD
<<<<<<< HEAD
    pub fn insert(&mut self, new_fd: usize, file: Arc<dyn FileOp>) -> Option<Arc<dyn FileOp>> {
=======
    pub fn insert(&self, new_fd: usize, file: Arc<dyn FileOp>) -> Option<Arc<dyn FileOp>> {
>>>>>>> 8162fada35bdfa8533bc38451ef1b322d3374e58
=======
    pub fn insert(&self, new_fd: usize, file: Arc<dyn FileOp>) -> Option<Arc<dyn FileOp>> {
>>>>>>> 13a1da6b38b24cc7e0cb99a67db59f853588ac62
        self.table.lock().insert(new_fd, file)
    }
}
