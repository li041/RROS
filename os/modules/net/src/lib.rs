#![no_main]
#![no_std]

extern crate alloc;
mod listentable;
mod addr;

use alloc::vec;
use spin::Mutex;
use smoltcp::iface::SocketSet;

///将somltcp中的socketset封装到socketsetwrapper中
struct SocketSetWrapper<'a>(Mutex<SocketSet<'a>>);

impl<'a> SocketSetWrapper<'a> {
    pub fn new()->Self{
        SocketSetWrapper(Mutex::new(SocketSet::new(vec![])))
    }
}
