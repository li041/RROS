/*
 * @Author: Peter/peterluck2021@163.com
 * @Date: 2025-03-12 22:41:14
 * @LastEditors: Peter/peterluck2021@163.com
 * @LastEditTime: 2025-03-18 22:33:18
 * @FilePath: /os/modules/net/src/udp.rs
 * @Description: udp socket net modules
 * 
 * Copyright (c) 2025 by peterluck2021@163.com, All Rights Reserved. 
 */
use core::{sync::atomic::{AtomicBool, Ordering}, u8};
use log::{error, info, warn};
use smoltcp::{iface::SocketHandle, socket::udp::{self, BindError, RecvError, SendError}, wire::{IpEndpoint, IpListenEndpoint}};
use spin::rwlock::RwLock;
use async_utils::{get_waker, suspend_now, yield_now};

use crate::{addr::{to_ipendpoint, UNSPECIFIED_LISTEN_ENDPOINT}, NetPollState};

use super::{err::SysError, portmap::PORT_MAP, SocketSetWrapper, SOCKET_SET};



pub struct UdpSocket{

    //handle 在被加入socketset之后就可以保留，作为socket唯一凭证
    handle:SocketHandle,
    //local_address and port.使用Rwlock保证多线程下读写安全,为什么使用Iplistenendpoint?
    local_addr:RwLock<Option<IpListenEndpoint>>,
    //peer_address and port 远程.使用Rwlock保证多线程下读写安全,为什么使用Ipendpoint?
    peer_addr:RwLock<Option<IpEndpoint>>,
    //是否阻塞,默认为false
    //选择Atomic,较Mutex更快，较Rwlock更有效，具有内部可变性
    nonblock:AtomicBool,
}

impl UdpSocket {
    pub fn new()->Self{
        let socket=SocketSetWrapper::new_udp_socket();
        let handle=SOCKET_SET.add(socket);
        UdpSocket{
            handle:handle,
            local_addr:RwLock::new(None),
            peer_addr:RwLock::new(None),
            nonblock:AtomicBool::new(false)
        }        
    }
    ///return local_addr
    pub fn local_addr(&self)->Result<IpEndpoint,SysError> 
    {
        match  self.local_addr.try_read(){
            Some(local)=>local.ok_or(SysError::ENOTCONN).map(to_ipendpoint),
            None=>Err(SysError::ENOTCONN)
        }
    }
    ///return peer_addr
    pub fn peer_addr(&self)->Result<IpEndpoint,SysError>{
        self.remote_endpoint()
    }

    /// Returns whether this socket is in nonblocking mode.
    pub fn is_nonblocking(&self)->bool {
    // Loads a value from the bool.
    //
    // `load` takes an [`Ordering`] argument which describes the memory ordering
    // of this operation. Possible values are [`SeqCst`], [`Acquire`] and [`Relaxed`].
    //
    // # Panics
    //
    // Panics if `order` is [`Release`] or [`AcqRel`].
    //
    // # Examples
    //
    // ```
    // use std::sync::atomic::{AtomicBool, Ordering};
    //
    // let some_bool = AtomicBool::new(true);
    //
    // assert_eq!(some_bool.load(Ordering::Relaxed), true);
    // ```
    // Acquire make sure the task behind it will defintely behind the task
        self.nonblock.load(Ordering::Acquire)
    }


    /// Moves this UDP socket into or out of nonblocking mode.
    ///
    /// This will result in `recv`, `recv_from`, `send`, and `send_to`
    /// operations becoming nonblocking, i.e., immediately returning from their
    /// calls. If the IO operation is successful, `Ok` is returned and no
    /// further action is required. If the IO operation could not be completed
    /// and needs to be retried, an error with kind
    /// [`Err(WouldBlock)`](AxError::WouldBlock) is returned.
    /// set udpsocket block for whether to block
    pub fn set_nonblocking(&self,nonblock:bool) {
        //Release make sure the constuction in front of this instruction will defintely in front of this instructions
        self.nonblock.store(nonblock, Ordering::Release);
    }
    pub fn connect(&self, addr: IpEndpoint) -> Result<(),SysError> {
        if self.local_addr.read().is_none() {
            info!(
                "[UdpSocket::connect] don't have local addr, bind to UNSPECIFIED_LISTEN_ENDPOINT"
            );
            self.bind(UNSPECIFIED_LISTEN_ENDPOINT)?;
        }
        let mut self_peer_addr = self.peer_addr.write();
        *self_peer_addr = Some(addr);
        info!(
            "[UdpSocket::connect] handle {} local {} connected to remote {}",
            self.handle,
            self.local_addr.read().as_ref().unwrap(),
            addr
        );
        Ok(())
    }


    /// check whether a socket is ready to bind a IPendpoint return bind
    pub fn check_bind(&self,fd:usize,bound_addr:IpListenEndpoint)->Option<usize> {
        match PORT_MAP.lock().get(bound_addr.port) {
            //if port has been used
            Some((fd,addr))=>{
                //has connected, the same
                if addr==bound_addr {
                    warn!("the {} port has been used by this ip address{}",bound_addr.port,bound_addr.port);
                    return Some(fd);
                }
                else {
                    //not same
                    todo!("the port in bound_addr {} has the ippoint not same as {}",addr,bound_addr);
                }
            }
            None=>{
                //no one use ,alloc one and can use it
                PORT_MAP.lock().insert(bound_addr.port, fd, bound_addr);
                return None;
            }
        }
        
    }
    //use socketset to bind a addr ,let local addr is same as bind addr
    pub fn bind(&self,bind_addr:IpListenEndpoint)->Result<(),SysError>{
        let mut local_addr=self.local_addr.write();
        if local_addr.is_some() {
            //has already bind to local addr
            error!("has already bind to a local addr");
            return Err(SysError::EINVAL);
        }
        SOCKET_SET.with_mut_socket::<udp::Socket,_,_>(self.handle, |socket|{
            // Bind the socket to the given endpoint.
            //
            // This function returns `Err(Error::Illegal)` if the socket was open
            // (see [is_open](#method.is_open)), and `Err(Error::Unaddressable)`
            // if the port in the given endpoint is zero.
            socket.bind(bind_addr).map_err(|e|{
                //indicate that will not exist?maybe
                match e {
                    BindError::InvalidState=>SysError::EEXIST,//indicate the socket has been bind
                    BindError::Unaddressable=>SysError::EINVAL//indicate the addr is zero
                }
            })
        });
        *local_addr=Some(bind_addr);
        info!("the udp socket{} is bind to addr{}",self.handle,bind_addr);
        Ok(())
    }
    pub async fn send_to(&self,buf:&[u8],remote_addr:IpEndpoint)->Result<usize,SysError> {
        if remote_addr.port == 0 || remote_addr.addr.is_unspecified() {
            warn!("socket send_to() failed: invalid remote address");
            return Err(SysError::EINVAL);
        }
        self.send_buf(buf,remote_addr).await
    }
    pub async fn send(&self, buf: &[u8]) -> Result<usize,SysError> {
        let remote_endpoint = self.remote_endpoint()?;
        self.send_buf(buf, remote_endpoint).await
    }
    pub async fn recv_from(&self,buf:&mut [u8])->Result<usize,SysError> {
        //buf is space to recv 
        self.recv_buf(buf).await
    }

    pub fn shutdown(&self)->Result<(),SysError> {
        SOCKET_SET.with_mut_socket::<udp::Socket,_,_>(self.handle, |socket|{
            info!("socket {} is going to close",self.handle);
            socket.close();
        });
        let timestamp=SOCKET_SET.poll_interface();
        SOCKET_SET.check_poll(timestamp);
        Ok(())
    }

    pub async fn poll(&self)->NetPollState {
        let mut recvable=false;
        let mut sendable=false;

        //not bind to local addr
        if self.local_addr.read().is_none() {
            return NetPollState{
                recvable:false,
                sendable:false,
                hangup:false
                };
        }
        let waker=get_waker().await;
        SOCKET_SET.with_mut_socket::<udp::Socket,_,_>(self.handle, |socket|{
            recvable=socket.can_recv();
            sendable=socket.can_send();
            if !recvable {
                //socket can not recv
                socket.register_recv_waker(&waker);
            }
            if !sendable {
                socket.register_send_waker(&waker);
                
            }
            NetPollState{
                recvable:recvable,
                sendable:sendable,
                hangup:false
            }
        })
        
    }

}

impl UdpSocket {
    fn remote_endpoint(&self)->Result<IpEndpoint,SysError> {
        match self.peer_addr.try_read() {
            Some(addr)=>addr.ok_or(SysError::ENOTCONN),
            None=>Err(SysError::ENOTCONN)   
        }
        
    }

    
    async fn send_buf(&self,buf:&[u8],remote_addr:IpEndpoint)->Result<usize,SysError> {
        //判断是否bind
        if self.local_addr.read().is_none() {
            //the socket is not bind to any addr
            error!("the socket {} is not bind to any addr",self.handle);
            //no bind,then we bind a unspecfied ipv4 endpoint
            self.bind(UNSPECIFIED_LISTEN_ENDPOINT);
        }
        //get a waker use to wake up thread
        let waker=get_waker().await;
        //check if is the block on mode and send the data
        let bytes=self.block_on(||{
            SOCKET_SET.with_mut_socket::<udp::Socket,_,_>(self.handle, |socket|{
                //get handle socket
                if socket.can_send() {
                    //send buf to local addr a function in smoltcp in udp
                    socket.send_slice(buf,remote_addr).map_err(|e|{
                        match e {
                            SendError::BufferFull=>{
                                //net device send buffet is full then we register a send waker use the waker we get.next time we use wake to continue run
                                socket.register_send_waker(&waker);
                                return SysError::EAGAIN;
                            },
                            SendError::Unaddressable=>{
                                //connection failed
                                return SysError::ECONNREFUSED;
                            }
                        }
                    });
                    Ok(buf.len())                    
                }
                else {
                    //send buffer in device is full
                    info!("socket handle {} can not send right now",self.handle);
                    socket.register_send_waker(&waker);
                    Err(SysError::EAGAIN)
                }
            })
        }).await.unwrap();
        info!("socket {} send {} bytes to {}",self.handle,bytes,remote_addr);
        yield_now().await;
        Ok(bytes)
    }

    async fn recv_buf(&self,buf:&mut [u8])->Result<usize,SysError> 
    {
        if self.local_addr.read().is_none() {
            error!("recv socket {:?},local_addr is none",self.local_addr());
            return Err(SysError::ENOTCONN);
        }

        let waker=get_waker().await;
        
        let bytes =self.block_on(||{
            SOCKET_SET.with_mut_socket::<udp::Socket,_,_>(self.handle, |socket|{
                if socket.can_recv() {
                    socket.recv_slice( buf).map_err(|e|{
                        match e {
                            RecvError::Exhausted=>{
                                //data has been read out
                                socket.register_recv_waker(&waker);
                                return SysError::EAGAIN;
                            }
                        }
                    }).unwrap();
                   Ok(buf.len())
                }
                else {
                    socket.register_recv_waker(&waker);
                    Err(SysError::EAGAIN)
                }
            })
        }).await;
        yield_now().await;
        bytes
   }
    //reture a future task
    /// block other task until this task is finished
    /// f is function task need block on 
    async fn block_on<F,T>(&self,mut f:F)->Result<T,SysError> 
    where F:FnMut()->Result<T,SysError>
    {
        //no block then complete
        if self.is_nonblocking() {
            f()
        }
        else {
            loop {
                //block mode
                let timestamp=SOCKET_SET.poll_interface();
                SOCKET_SET.check_poll(timestamp);
                let ret = f();
                match ret {
                    Ok(t)=>return Ok(t),
                    //try_again
                    Err(SysError::EAGAIN)=>{
                        //暂时挂起，执行其他内容
                        suspend_now().await;
                        error!("block on error on source no ready,we will try again");
                    }
                    _=>return Err(SysError::EINTR)
                    
                }
            }
        }
    }
    
}