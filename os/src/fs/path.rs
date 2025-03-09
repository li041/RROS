use alloc::sync::Arc;

use super::{dentry::Dentry, mount::VfsMount};

pub struct Path {
    pub mnt: Arc<VfsMount>,
    pub dentry: Arc<Dentry>,
}

impl Path {
    pub fn zero_init() -> Arc<Self> {
        Arc::new(Path {
            mnt: Arc::new(VfsMount::zero_init()),
            dentry: Arc::new(Dentry::zero_init()),
        })
    }
<<<<<<< HEAD
<<<<<<< HEAD
=======
=======
>>>>>>> 13a1da6b38b24cc7e0cb99a67db59f853588ac62
    pub fn from_existed_user(old_path: &Arc<Path>) -> Arc<Self> {
        Arc::new(Path {
            mnt: old_path.mnt.clone(),
            dentry: old_path.dentry.clone(),
        })
    }
<<<<<<< HEAD
>>>>>>> 8162fada35bdfa8533bc38451ef1b322d3374e58
=======
>>>>>>> 13a1da6b38b24cc7e0cb99a67db59f853588ac62
    pub fn new(mnt: Arc<VfsMount>, dentry: Arc<Dentry>) -> Arc<Self> {
        Arc::new(Path { mnt, dentry })
    }
}
