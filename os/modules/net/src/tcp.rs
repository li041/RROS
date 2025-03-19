
/*
 * @Author: Peter/peterluck2021@163.com
 * @Date: 2025-03-15 10:26:06
 * @LastEditors: Peter/peterluck2021@163.com
 * @LastEditTime: 2025-03-19 14:34:37
 * @FilePath: /os/modules/net/src/tcp.rs
 * @Description: tcp net module
 * 
 * Copyright (c) 2025 by peterluck2021@163.com, All Rights Reserved. 
 */
use async_utils::{get_waker, suspend_now, yield_now};
use log::{error, info, warn};
use smoltcp::{self, iface::SocketHandle, socket::tcp::{self, ConnectError, RecvError, SendError}, wire::{IpAddress, IpEndpoint, IpListenEndpoint}};
use spin::Mutex;
use core::{cell::UnsafeCell,sync::atomic::{AtomicBool, AtomicU8, Ordering}};
use crate::{addr::{UNSPECIFIED_IPV4_ENDPOINT, UNSPECIFIED_IPV6_ENDPOINT}, TCP_SEND_BUFFER};
use super::{ETH0,addr::UNSPECIFIED_IPV4, err::SysError, SOCKET_SET,SocketSetWrapper,listentable::LISTEN_TABLE};


pub const SHUT_RD: u8 = 0;
pub const SHUT_WR: u8 = 1;
pub const SHUT_RDWR: u8 = 2;

/// 表示读方向已关闭（相当于SHUT_RD）
pub const RCV_SHUTDOWN: u8 = 1;
/// 表示写方向已关闭（相当于SHUT_WR）
pub const SEND_SHUTDOWN: u8 = 2;
/// 表示读和写方向都已关闭（相当于SHUT_RDWR）
pub const SHUTDOWN_MASK: u8 = 3;


/// send FIN for closed
const STATE_CLOSED: u8 = 0;

const STATE_BUSY: u8 = 1;
/// 3 time for tcp connecting
const STATE_CONNECTING: u8 = 2;
//has connected
const STATE_CONNECTED: u8 = 3;
/// 
const STATE_LISTENING: u8 = 4;
pub struct TcpSocket{
    /// socket hanle
    handle:UnsafeCell<Option<SocketHandle>>,
    /// bind local addr
    local_addr:UnsafeCell<Option<IpEndpoint>>,
    /// connected addr 
    peer_addr:UnsafeCell<Option<IpEndpoint>>,
    /// block or not change this will keep in an atomic instruction
    block:AtomicBool,
    /// tcp is based on connection,so we need to build state model to choose state
    state:AtomicU8,
    /// Indicates whether the read or write directions of the socket have been
    /// explicitly shut down. This does not represent the connection state.
    /// Once shut down, the socket cannot be reconnected via `connect`.
    shutdown:UnsafeCell<u8>
}

/// public function
impl TcpSocket {
    pub const fn new()->Self {
        TcpSocket{
            handle:UnsafeCell::new(Option::None),
            local_addr:UnsafeCell::new(Some(UNSPECIFIED_IPV4_ENDPOINT)),
            peer_addr:UnsafeCell::new(Some(UNSPECIFIED_IPV6_ENDPOINT)),
            block:AtomicBool::new(false),
            state:AtomicU8::new(STATE_CLOSED),
            shutdown:UnsafeCell::new(0)
        }
    }

    /// get a new socket with handle peer_addr,local_addr,and we use state for not closed
    /// use shutdown for 0,use block for false
    fn new_socket_with_addr(handle:SocketHandle,peer_addr:IpEndpoint,local_addr:IpEndpoint)->Self {
        TcpSocket{
            handle:UnsafeCell::new(Some(handle)),
            local_addr:UnsafeCell::new(Some(local_addr)),
            peer_addr:UnsafeCell::new(Some(peer_addr)),
            state:AtomicU8::new(STATE_CLOSED),
            block:AtomicBool::new(false),
            shutdown:UnsafeCell::new(0)
        }
    }

    pub fn local_addr(&self)->Result<IpEndpoint,SysError> {
        match self.get_state() {
            STATE_CONNECTED|STATE_LISTENING|STATE_CLOSED=>{
                 Ok(unsafe { self.local_addr.get().read().unwrap() }) 
            }
            _=>{
                Err(SysError::ENOTCONN)
            }
        }
    }
    pub fn peer_addr(&self)->Result<IpEndpoint,SysError>{
        match self.get_state() {
            STATE_CONNECTED|STATE_LISTENING=>{
                Ok(unsafe { self.peer_addr.get().read().unwrap() })
            }
            _=>Err(SysError::ENOTCONN)
        }
    }
    pub fn is_nonblocking(&self)->bool{
        self.block.load(Ordering::Acquire)
    }

    /// Moves this TCP stream into or out of nonblocking mode.
    ///
    /// This will result in `read`, `write`, `recv` and `send` operations
    /// becoming nonblocking, i.e., immediately returning from their calls.
    /// If the IO operation is successful, `Ok` is returned and no further
    /// action is required. If the IO operation could not be completed and needs
    /// to be retried, an error with kind
    /// [`Err(WouldBlock)`](AxError::WouldBlock) is returned.
    pub fn set_nonblocking(&self,block:bool){
        self.block.store(block, Ordering::Release);
    }

    /// connect to a peer_addr
    /// 
    pub async fn connect(&self,peer_addr:IpEndpoint)->Result<(),SysError> {
        // wait for anther complete
        yield_now().await;
        // now we need to change state to connect
        // if state is closed we use function if return ok then we change state to connecting
        self.update_state(STATE_CLOSED, STATE_CONNECTING, ||{
            //if is some use function f
            let handle=unsafe { self.handle.get().read() }.unwrap_or_else(||{
                SOCKET_SET.add(SocketSetWrapper::new_tcp_socket())
            });
            // socket local addr
            let bound_point=self.bound_point()?;
            // netdevice 0
            let iface=&ETH0.get().unwrap().iface;
            // use smoltcp connect return connected remote addr and local addr with dault then bound addr is local addr
            let (local_endpoint,remote_endpoint)=SOCKET_SET.with_mut_socket::<tcp::Socket,_,_>(handle, |socket|{
                socket.connect(iface.lock().context(), peer_addr, bound_point).or_else(|e|{
                        match e {
                            ConnectError::InvalidState=>{
                            // not state_closed
                            // When attempting to perform an operation, the socket is in an
                            // invalid state. Such as attempting to call the connection operation
                            // again on an already connected socket, or performing
                            // the operation on a closed socket
                            // if socket is in connected then it can not connected
                            Err(SysError::EBADF)
                            },
                            ConnectError::Unaddressable=>{
                                // peeraddr can not connect
                                Err(SysError::EADDRNOTAVAIL)
                            }
                        }
                    })?;
                Ok((socket.local_endpoint().unwrap(),socket.remote_endpoint().unwrap()))
            })?;
            unsafe { 
                self.local_addr.get().write(Some(local_endpoint));
                self.peer_addr.get().write(Some(remote_endpoint));
                self.handle.get().write(Some(handle));
            };
            Ok(())
        }).unwrap_or_else(|_|{
            error!("[TcpSocket::connect] failed: already connected");
            Err(SysError::EEXIST)
        })
       // Here our state must be `CONNECTING`, and only one thread can run here.
    //    if self.is_nonblocking() {
    //     Err(SysError::EINPROGRESS)
    //     } else {
    //     self.block_on_async(|| async {
    //         let NetPollState { , .. } = self.poll_connect().await;
    //         if !writable {
    //             warn!("[TcpSocket::connect] failed: try again");
    //             Err(SysError::EAGAIN)
    //         } else if self.get_state() == STATE_CONNECTED {
    //             Ok(())
    //         } else {
    //             warn!("[TcpSocket::connect] failed, connection refused");
    //             Err(SysError::ECONNREFUSED)
    //         }
    //     })
    //     .await
    //     }
}
    
    /// bind local addr to socket local addr
    /// write local_addr to socket localaddr
    pub fn bind(&self,mut local_addr:IpEndpoint)->Result<(),SysError> {
        self.update_state(STATE_CLOSED, STATE_CLOSED, ||{
            // TODO:check the addr is valid
            if local_addr.port==0 {
                // local_addr don not has valid port
                // alloc one
                let port=get_ephemeral_port().unwrap();
                local_addr.port=port;
            }
            unsafe {
                let old_addr=self.local_addr.get().read();
                // if old_addr.is_none() {
                //     self.local_addr.get().write(local_addr);
                // }
                // else {
                //     Err(SysError::EADDRINUSE)
                // }
                if old_addr!=Some(UNSPECIFIED_IPV4_ENDPOINT) {
                    // socket local addr has been used 
                    // 
                    return Err(SysError::EADDRINUSE);
                }
                if let IpAddress::Ipv6(ipv6)=local_addr.addr 
                {
                    if ipv6.is_unspecified() {
                        local_addr.addr=UNSPECIFIED_IPV4
                    }
                }
                self.local_addr.get().write(Some(local_addr));
            }
            Ok(())
        }).unwrap_or_else(|_|{
            Err(SysError::EINVAL)
        })
    }

    /// start listening on bind/local addr port
    pub fn listen(&self)->Result<(),SysError> {
        self.update_state(STATE_CLOSED, STATE_LISTENING,||{
            let bind_endpoint=self.bound_point()?;
            // pub self in listen tabel port entry socket set 
            LISTEN_TABLE.listen(bind_endpoint)?;
            unsafe {
                info!("socekt {:?} listening on ippoint {:?}",self.handle.get().read(),bind_endpoint);
            }
            Ok(())
        }).unwrap_or(Ok(()))
    }


    
    /// Accepts a new connection.
    ///
    /// This function will block the calling thread until a new TCP connection
    /// is established. When established, a new [`TcpSocket`] is returned.
    ///
    /// It's must be called after [`bind`](Self::bind) and
    /// [`listen`](Self::listen).
    pub async fn accept(&self)->Result<TcpSocket,SysError> {
        //examine whether socket is bind and listen
        if !self.is_listening() {
            //not listening can not accept
            return Err(SysError::EINVAL);
        }
        let port=unsafe { self.local_addr.get().read().unwrap().port };
        self.block_on(||{
            // according socket local addr port in listentabel entry search a socket listening on this port waiting to connect
            // return peer and local addr
            // front of this connect and bind has writing local and remote addr into socekt
            let (sockethandle,(local_addr,peer_addr))=LISTEN_TABLE.accept(port)?;
            //TODO this socket will be same? the new socket will be same with old socket
            Ok(TcpSocket::new_connected(sockethandle, local_addr, peer_addr))
        }).await
    }

    /// close the connection
    pub fn shutdown(&self,method:u8)->Result<(),SysError> {
    unsafe {       
                let shutdown=self.shutdown.get();
                match method {
                    SHUT_RD=>*shutdown|=RCV_SHUTDOWN,
                    SHUT_WR=>*shutdown|=SEND_SHUTDOWN,
                    SHUT_RDWR=>*shutdown|=SHUTDOWN_MASK,
                    _=>return Err(SysError::EINVAL)
                }
            }

        self.update_state(STATE_CONNECTED, STATE_CLOSED, ||{
            let handle=unsafe { self.handle.get().read().unwrap() };
            SOCKET_SET.with_mut_socket::<tcp::Socket,_,_>(handle, |socket|{
                warn!("socket {} is going to shutdown,before state is {}",handle,socket.state());
                socket.close();
                warn!("socket {} is already shutdown,state is {}",handle,socket.state());
            });
            let timestamp=SOCKET_SET.poll_interface();
            SOCKET_SET.check_poll(timestamp);
            Ok(())
        }).unwrap_or(Ok(()))?;
        self.update_state(STATE_LISTENING, STATE_CLOSED, ||{
            let local_addr=unsafe { self.local_addr.get().read() };
            let port=local_addr.unwrap().port;
            //delete the entry
            LISTEN_TABLE.unlisten(port);
            unsafe { self.local_addr.get().write(Some(UNSPECIFIED_IPV4_ENDPOINT)) };
            let timestamp=SOCKET_SET.poll_interface();
            SOCKET_SET.check_poll(timestamp);
            Ok(())
        }).unwrap_or(Ok(()))?;
        Ok(())
    }


    /// send buf to remote addr which has connect and accept 
    pub async fn send(&self,buf:&[u8])->Result<usize,SysError> {

        // let shutdown = unsafe { *self.shutdown.get() };
        // if shutdown & SEND_SHUTDOWN != 0 {
        //     log::warn!("[TcpSocket::send] shutdown closed write, send return 0");
        //     return Ok(0);
        // }
        //examine the state must be connected
        if self.is_connecting() {
            //no connecting
            return Err(SysError::EAGAIN);
        }
        // } else if !self.is_connected() && shutdown == 0 {
        //     warn!("socket send() failed");
        //     return Err(SysError::ENOTCONN);
        // }
        let handle=unsafe { self.handle.get().read().unwrap() };
        let waker=get_waker().await;

        // use block on build a futurn task let send task async complete
        let ret=self.block_on(||{
            SOCKET_SET.with_mut_socket::<tcp::Socket,_,_>(handle, |socket|{
                // make sure that remote connection is active and can make sure that the data can arrive at remote addr
                if !socket.is_active()|!socket.may_send() {
                    error!("fail to send the connection is not active");
                    Err(SysError::ENOTCONN)
                }
                else if socket.can_send() {
                    let len=socket.send_slice(buf).map_err(|e|{
                        match e {
                            SendError::InvalidState=>{
                                SysError::EAGAIN;
                            }
                        }
                    }).unwrap();
                    Ok(len)
                }
                else {
                    // device send buf is full we need wait register a send waker
                    socket.register_send_waker(&waker);
                    Err(SysError::EAGAIN)
                }
            })
        }).await;
        if let Ok(bytes) = ret {
            if bytes>TCP_SEND_BUFFER/2 {
                
            }
            else {
                yield_now().await;
            }
        }
        SOCKET_SET.poll_interface();
        ret
    }

    
    //recv buf
    pub async fn recv(&self,buf:&mut [u8])->Result<usize,SysError>{
        let waker=get_waker().await;
        //examine state ,not for connecting,only for connect
        let ret=self.block_on(||{
            let handle=unsafe { self.handle.get().read().unwrap() };
            SOCKET_SET.with_mut_socket::<tcp::Socket,_,_>(handle, |socket|{
                if !socket.is_active() ||!socket.may_recv(){
                    error!("fail to recv for connnection is not active");
                    Err(SysError::ENOTCONN)
                }
                else if socket.can_recv() || socket.recv_queue()>0{
                    let len=socket.recv_slice( buf).map_err(|e|{
                        match  e {
                            RecvError::Finished=>{
                                //connection is close cannot recv data
                                SysError::EAGAIN;
                            },
                            RecvError::InvalidState=>{
                                // i think with state check in front of this can not happen
                                error!("happen for invalidstate at recv data");
                                SysError::EAGAIN;
                            }
                        }
                    }).unwrap();
                    Ok(len)
                }
                else {
                    //device recv full of data
                    socket.register_recv_waker(&waker);
                    Err(SysError::EAGAIN)
                }
            })
        }).await;
        ret
    }


    pub async fn poll() {
        
        
    }

}



/// no use
/// 
// pub struct TcpRecvFuture<'a>{
//     socket:&'a TcpSocket,
//     buf:&'a [u8]
// }

// impl <'a> Future for TcpRecvFuture {
//     type Output=result;

//     /// check in roll
//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         let shutdown=unsafe { self.socket.shutdown.get().read() };
//         if shutdown&RCV_SHUTDOWN!=0 {
//             info!("[TcpRecvFuture] socket close the read recv ");
//             return Poll::Ready(Ok(0))
//         }
//         if self.socket.is_connecting() {
//             info!("[TcpRecvFuture] may loss waker");
//             return Poll::Ready(Ok(0));
//         }
//         else if !self.socket.is_connected()&&shutdown==0 {
//             info!("[TcpRecvFuture] socket connection is built");
//             return  Poll::Ready(Err(SysError::ENOTCONN));            
//         }

//         let handle=unsafe { self.socket.handle.get().read() };
//         let ret=SOCKET_SET.with_mut_socket::<tcp::Socket>(handle, |socket|{
//             info!("[TcpRecvFuture] socket {} state is {}",handle,socket.state());
//             if !socket.is_active() {
//                 info!("[TcpRecvFuture] socket {} connection is invalid",handle);
//                 Poll::Ready(Err(SysError::ENOTCONN))
//             }
//             else if !socket.may_recv() {
//                 Poll::Ready(Ok(0))                
//             }
//             else if socket.recv_queue()>0 {
//                 Poll::Ready(Ok(0))   
//             }
//             else {
//                 info!("socket {} don not has enough data to send",handle);
//                 if self.socket.is_nonblocking() {
//                     return Poll::Ready(Err(SysError::EAGAIN));
//                 }
//                 else {
//                     socket.register_recv_waker(cx.waker());
//                     return Poll::Pending;
//                 }
//             }
//         });
//         SOCKET_SET.poll_interface();
//         ret
//     }
// }





/// private function
impl TcpSocket {
    fn get_state(&self)->u8 {
        self.state.load(Ordering::Acquire)
    }
    fn set_state(&self,state:u8) {
        self.state.store(state, Ordering::Release);
    }

    /// Update the state of the socket atomically.
    ///
    /// If the current state is `expect`, it first changes the state to
    /// `STATE_BUSY`, then calls the given function. If the function returns
    /// `Ok`, it changes the state to `new`, otherwise it changes the state
    /// back to `expect`.
    ///
    /// It returns `Ok` if the current state is `expect`, otherwise it returns
    /// the current state in `Err`.
    fn update_state<F,T>(&self,expect:u8,new:u8,f:F)->Result<Result<T,SysError>,u8>
    where F:FnOnce()->Result<T,SysError>
    {
        match self.state.compare_exchange(expect, STATE_BUSY, Ordering::Acquire, Ordering::Acquire) {
            Ok(_)=>{
                let ret=f();
                if ret.is_ok() {
                    self.set_state(new);
                }
                else if ret.is_err() {
                    self.set_state(expect);
                }
                Ok(ret)
            },
            // not equal to expect return state with err
            Err(old)=>Err(old)
        }
    }
    fn is_connecting(&self)->bool {
        self.state.load(Ordering::Acquire)==STATE_CONNECTING
    }
    fn is_connected(&self)->bool {
        self.state.load(Ordering::Acquire)==STATE_CONNECTED
    }
    fn is_listening(&self)->bool {
        self.state.load(Ordering::Acquire)==STATE_LISTENING
    }
    fn is_busy(&self)->bool {
        self.state.load(Ordering::Acquire)==STATE_BUSY
    }
    fn is_closed(&self)->bool {
        self.state.load(Ordering::Acquire)==STATE_CLOSED
    }

    async fn block_on<F,T>(&self,mut f:F)->Result<T,SysError> 
    where F:FnMut()->Result<T,SysError>
    {
        if self.is_nonblocking() {
            f()
        }
        else {
            loop {
                let timestamp=SOCKET_SET.poll_interface();
                SOCKET_SET.check_poll(timestamp);
                let ret=f();
                match ret {
                    Ok(t)=>return Ok(t),
                    Err(SysError::EAGAIN)=>{
                        suspend_now().await;
                    }
                    Err(e)=>return Err(e)
                }
            }
        }
    }
    /// from local addr return a valid addr and port 
    /// bound addr should be local addr
    fn bound_point(&self)->Result<IpListenEndpoint,SysError> {
        let local_point=unsafe { self.local_addr.get().read() };
        let port=local_point.unwrap().port;
        assert_ne!(port,0);
        let addr=if local_point.unwrap().addr!=UNSPECIFIED_IPV4 {
            Some(local_point.unwrap().addr)
        }else{
            None
        };
        Ok(IpListenEndpoint{
            addr:addr,
            port:port
        })
    }
    fn new_connected(handle:SocketHandle,local_addr:IpEndpoint,remote_addr:IpEndpoint,
    ) ->Self{
        TcpSocket{
            handle:UnsafeCell::new(Some(handle)),
            local_addr:UnsafeCell::new(Some(local_addr)),
            peer_addr:UnsafeCell::new(Some(remote_addr)),
            block:AtomicBool::new(false),
            state:AtomicU8::new(STATE_CONNECTED),
            shutdown:UnsafeCell::new(0)
        }
    }
    // Poll the status of a TCP connection to determine if it has been
    // established (successful connection) or failed (closed connection)
    //
    // Returning `true` indicates that the socket has entered a stable
    // state(connected or failed) and can proceed to the next step
    // async fn poll_connect(&self)->NetPollState {
    //     ///readalbe=>recv
    //     /// writeable=>send
    //     let handle=unsafe { self.handle.get().read() };
    //     SOCKET_SET.with_mut_socket::<tcp::Socket,_,_>(handle, |socket|{
    //         match  {
                
    //         }
    //     })
        
    // }

}


fn get_ephemeral_port()->Result<u16,SysError> {
    const PORT_START: u16 = 0xc000;
    const PORT_END: u16 = 0xffff;
    static CURR:Mutex<u16>=Mutex::new(PORT_START);

    let mut curr=CURR.lock();
    let  tries=0;
    while tries<=PORT_END {
        let port: u16=*curr;
        let listen_endpoint:IpListenEndpoint=port.into();
        // use can_listen to identify the port entry is none or has been use
        if LISTEN_TABLE.can_listen(listen_endpoint.port){
            return Ok(port);
        }
        if *curr==PORT_END {
            *curr=PORT_START
        }
        else {
            *curr+=1;
        }
    }
    Err(SysError::EADDRINUSE)

    
}