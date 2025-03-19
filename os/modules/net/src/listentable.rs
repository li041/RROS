/*
 * @Author: Peter/peterluck2021@163.com
 * @Date: 2025-03-11 16:59:55
 * @LastEditors: Peter/peterluck2021@163.com
 * @LastEditTime: 2025-03-19 14:34:28
 * @FilePath: /os/modules/net/src/listentable.rs
 * @Description: net listentable modules
 * 
 * Copyright (c) 2025 by peterluck2021@163.com, All Rights Reserved. 
 */
// use core::task::Waker;
use alloc::{boxed::Box, collections::vec_deque::VecDeque};
use log::{info, warn,error};
use smoltcp::{iface::{SocketHandle, SocketSet}, socket::tcp::{self, State}, wire::{IpAddress, IpEndpoint, IpListenEndpoint}};
use spin::{mutex::Mutex, Lazy};

// use super::SyncFlag;

use super::{err::SysError, SocketSetWrapper};

use super::SOCKET_SET;

const LISTEN_QUEUE_SIZE: usize = 512;
const PORT_NUM:usize=65536;

pub static LISTEN_TABLE:Lazy<ListenTable>=Lazy::new(ListenTable::new);


//listen table中的entry,，包括监听的ip地址和port端口
struct ListenTableEntry{
    //此entry监听的ip地址和port
    // pub struct ListenEndpoint {
    //     pub addr: Option<Address>,
    //     pub port: u16,
    // }
    listen_endpoint:IpListenEndpoint,
    // listen tabel identify socket listening on this ippoint
    syn_queue:VecDeque<SocketHandle>,
    //当一个连接请求arrive时，唤醒一个监听socket
    // wake:Waker
}

impl ListenTableEntry {
    pub fn new(listen_endpoint:IpListenEndpoint)->Self{
        ListenTableEntry{
            listen_endpoint:listen_endpoint,
            syn_queue:VecDeque::with_capacity(LISTEN_QUEUE_SIZE),
        }
    }

    //一个listentabel entry可以接受的标准有2
    //1. 要么连接的地址和监听的地址一样,ipv4,ipv6均一样，则可以连接
    //2. 要么ipv6 mapped ipv4,这要求，输入为ipv4，且ipv6为ffff:a:b:c:d格式其中a:b:c:d与ipv4一样
    //3. 监听地址为ipv6 0:0:0:0:0:0则接受任何连接
    fn can_accept(&self,addr:IpAddress)->bool{
        match self.listen_endpoint.addr {

            Some(ip)=>{
                //the same
                if ip==addr {
                    return true;
                }
                //ipv6 is mapped ipv6,examine ipv6 address,examine ipv4 address
                if let IpAddress::Ipv6(ipv6) = ip {
                    if ip.is_unspecified() ||(ipv6.as_bytes()[12..]==addr.as_bytes()[..]&&ipv6.is_ipv4_mapped()&&addr.as_bytes().len()==4){
                       return true;
                    }
                }
                false
            }
            None=>false
        }
    }
    // pub fn wake(self) {
    // // Wakes up the task associated with this `Waker` without consuming the `Waker`.
    // //
    // // This is similar to [`wake()`](Self::wake), but may be slightly less efficient in
    // // the case where an owned `Waker` is available. This method should be preferred to
    // // calling `waker.clone().wake()`.
    //     self.wake.wake_by_ref();
    // }
}

impl Drop for ListenTableEntry {
    fn drop(&mut self) {
        for &handle in &self.syn_queue {
            SOCKET_SET.remove(handle);
        }
    }
}


// 定义由listenentry组成的监听表
// Mutex:确保多线程安全性，可能存在多个线程访问监听表
// Box:确保使用动态内存，这里的entrya应当可以不断变化
pub struct ListenTable{
    listentabelentry_queue:Box<[Mutex<Option<Box<ListenTableEntry>>>]>
}

impl ListenTable {
    pub fn new()->Self {
        let tcp = unsafe {
            let mut buf = Box::new_uninit_slice(PORT_NUM);
            for i in 0..PORT_NUM {
                buf[i].write(Mutex::new(None));
            }
            buf.assume_init()
        };
        Self { listentabelentry_queue:tcp }
    }
    /// 判断是否可以监听，如果对应的listen_entry为空则可，反之不可，意味着这个端口已经被监听了
    pub fn can_listen(&self,port:u16)->bool {
        self.listentabelentry_queue[port as usize].lock().is_none()
    }
    pub fn listen(&self,listen_endpoint:IpListenEndpoint)->Result<(),SysError>{
        let port=listen_endpoint.port;
        assert_ne!(port, 0);
        let _bool=self.can_listen(port);
        let mut entry=self.listentabelentry_queue[port as usize].lock();
        //已经占用了
        if entry.is_none() {
            *entry = Some(Box::new(ListenTableEntry::new(listen_endpoint)));
            Ok(())
        } else {
            warn!("socket listen() failed");
            Err(SysError::EADDRINUSE)
        }
    }
    //取消某个port的监听
    pub fn unlisten(&self,port:u16){
        info!("unlisten port {}",port);
        // if let Some(entry)=self.listentabelentry_queue[port as usize].lock().take(){
        //     *entry=None;
        // }
        // self.listentabelentry_queue
        *self.listentabelentry_queue[port as usize].lock()=None
    }
    pub fn can_accept(&self,port:u16)->bool{
        //如果存在，说明监听建立，下面检查是否存在connect,之后才可以accept
        if let Some(entry) = self.listentabelentry_queue[port as usize].lock().take() {
            entry.syn_queue.iter().any(|handle: &SocketHandle|{
                is_connected(*handle)
            })
        }
        else {
            false
        }
    }
    //检查端口上的SYN队列，找到已经建立连接的句柄，并将其从队列中取出
    //返回对应连接的socket,本地ip,用户ip
    pub fn accept(&self,port:u16)->Result<(SocketHandle,(IpEndpoint,IpEndpoint)),SysError> {
        if let Some(entry)=self.listentabelentry_queue[port as usize].lock().take(){
            let syn_queue=& mut entry.syn_queue.clone();
            let pair=syn_queue.iter().enumerate().find_map(|(index,&handle)|{
                    is_connected(handle).then(||(index,get_dst_addr(handle)))
            }).unwrap();
            //如果可能会存在由于网络阻塞导致每次得到的socket是后面的，意味着前面的socketo没有is_connect
            let handle=syn_queue.swap_remove_front(pair.0).unwrap();
            Ok((handle,(pair.1)))
        }else{
            Err(SysError::EAGAIN)
        }
    }



    pub fn incoming_tcp_packet(&self,src:IpEndpoint,dst:IpEndpoint,sockets:&mut SocketSet<'_>){
        if let Some(mut entry) = self.listentabelentry_queue[dst.port as usize].lock().take() {
            if !entry.can_accept(dst.addr) {
                //不连接，因为没有listen
                error!("不连接,因为没有listen");
            }
            if entry.syn_queue.len()>LISTEN_QUEUE_SIZE {
                //不放入待连接队列
                error!("不放入待连接队列");
            }
            let mut socket=SocketSetWrapper::new_tcp_socket();
            if socket.listen(entry.listen_endpoint).is_ok() {
                info!("socket tcp is going to connect from {:?} to {:?}",src,dst);
                let handle=sockets.add(socket);
                entry.syn_queue.push_back(handle);
            }
        }
    }
}
//a检查handleg对应的socket是否可以连接，前提是必须listen,connect
fn is_connected(handle:SocketHandle)->bool{
    SOCKET_SET.with_socket::<tcp::Socket, _, _>(handle, |socket| {
        !matches!(socket.state(), State::Listen | State::SynReceived)
    })
}
fn get_dst_addr(handle:SocketHandle)->(IpEndpoint,IpEndpoint){
    SOCKET_SET.with_socket::<tcp::Socket,_,_>(handle, |socket|{
        (socket.local_endpoint().unwrap(),socket.remote_endpoint().unwrap())
    })
}

