/*
 * @Author: Peter/peterluck2021@163.com
 * @Date: 2025-03-18 14:50:03
 * @LastEditors: Peter/peterluck2021@163.com
 * @LastEditTime: 2025-03-19 17:16:32
 * @FilePath: /os/src/net/socket.rs
 * @Description: 
 * 
 * Copyright (c) 2025 by peterluck2021@163.com, All Rights Reserved. 
 */

use core::any::Any;

use alloc::sync::Arc;
use downcast_arc::DowncastArc;
/*
 * @Author: Peter/peterluck2021@163.com
 * @Date: 2025-03-18 14:50:03
 * @LastEditors: Peter/peterluck2021@163.com
 * @LastEditTime: 2025-03-18 22:52:10
 * @FilePath: /os/src/net/socket.rs
 * @Description: 
 * 
 * Copyright (c) 2025 by peterluck2021@163.com, All Rights Reserved. 
 */
use log::{info, warn};
use net::{addr::UNSPECIFIED_IPV4, err::SysError, tcp::TcpSocket, udp::UdpSocket, IpEndpoint, NetPollState};
use crate::{fs::{dentry::Dentry, file::{ File, FileOp, O_NONBLOCK, O_RDWR}, path::Path}, syscall::fs::sys_dup2, task::current_task};
// use tokio::runtime::Runtime;
use super::{addr::SocketAddr, unix::UnixSocket, SaFamily, SocketType};
// use smol::block_on;
use async_utils::block_on;

pub enum Sock {
    Tcp(TcpSocket),
    Udp(UdpSocket),
    Unix(UnixSocket),    
}

impl Sock {
    pub fn set_nonblocking(&self) {
        match self {
            Self::Tcp(socket)=>socket.set_nonblocking(true),
            Self::Udp(socket)=>socket.set_nonblocking(true),
            Self::Unix(socket)=>unimplemented!()
        }
    }
    pub fn bind(&self,sockfd:usize,local_addr:SocketAddr)->Result<(),SysError> {
        match self {
            Self::Tcp(socket)=>{
                let end_point=local_addr.into_listen_endpoint();
                let addr=if end_point.addr.is_none() {
                    UNSPECIFIED_IPV4
                }else{
                    end_point.addr.unwrap()
                };
                socket.bind(IpEndpoint::new(addr,end_point.port))
            },
            Self::Udp(socket)=>{

                let end_point=local_addr.into_listen_endpoint();
                if let Some(prev_fd) = socket.check_bind(sockfd, end_point) {
                    //two socket with different fd can dup with fd when they has same addr and port
                   sys_dup2(prev_fd, sockfd);
                   return Ok(());
                }
                socket.bind(local_addr.into_listen_endpoint())
            },
            Self::Unix(socket)=>unimplemented!()
        }
    }
    pub fn listen(&self)->Result<(),SysError> {
        match self {
            Self::Tcp(socket)=>socket.listen(),
            Self::Udp(socket)=>Err(SysError::EOPNOTSUPP),
            Self::Unix(_)=>unimplemented!(),
        }
    }
    pub async fn accept(&self)->Result<TcpSocket,SysError> {
        //only tcp need to accept udp don not need to accept
        match self {
            Self::Tcp(socket)=>{
                let new_socket=socket.accept().await?;
                Ok(new_socket)
            },
            Self::Udp(socket)=>Err(SysError::EOPNOTSUPP),
            Self::Unix(_)=>unimplemented!()
        }
    }
    pub async fn connect(&self,remote_addr:SocketAddr)->Result<(),SysError> {
        match self {
            Self::Tcp(socket)=>{
                        let remote_endpoint=remote_addr.into_endpoint();
                        socket.connect(remote_endpoint).await
                    }
            Sock::Udp(udp_socket) => {
                let remote_endpoint=remote_addr.into_endpoint();
                udp_socket.connect(remote_endpoint)
            },
            Sock::Unix(unix_socket) => unimplemented!(),
        }
        
    }

    pub fn peer_addr(&self)->Result<SocketAddr,SysError>{
        match self {
            Self::Tcp(tcp)=>{
                Ok(SocketAddr::from_endpoint(tcp.peer_addr().unwrap()))
            }
            Self::Udp(udp)=>Ok(SocketAddr::from_endpoint(udp.peer_addr().unwrap())),
            Self::Unix(_)=>unimplemented!()
        }
    }
    pub fn local_addr(&self)->Result<SocketAddr,SysError>{
        match self {
            Self::Tcp(tcp)=>Ok(SocketAddr::from_endpoint(tcp.local_addr().unwrap())),
            Self::Udp(udp)=>Ok(SocketAddr::from_endpoint(udp.local_addr().unwrap())),
            Self::Unix(_)=>unimplemented!()
        }
    }

    pub async fn sendto(&self,buf:&[u8],remote_addr:Option<SocketAddr>)->Result<usize,SysError> {
        match self {
            Self::Tcp(tcp)=>{
                // let remote_endpoint=SocketAddr::into_endpoint(&remote_addr)
                tcp.send(buf).await
            }
            Self::Udp(udp)=>{
                match remote_addr {
                    Some(addr)=>{
                        udp.send_to(buf,SocketAddr::into_endpoint(&addr)).await
                    }
                    None=>udp.send(buf).await
                }
            }
            Self::Unix(_)=>unimplemented!()
        }   
    }
    pub async fn recvfrom(&self,buf:&mut [u8])->Result<(usize,SocketAddr),SysError> {
        match self {
            Self::Udp(udp)=>{
                let len=udp.recv_from(buf).await.unwrap();
                Ok((len,SocketAddr::from_endpoint(udp.peer_addr().unwrap())))
            },
            Self::Tcp(tcp)=>{
                let len=tcp.recv(buf).await.unwrap();
                Ok((len,SocketAddr::from_endpoint(tcp.peer_addr().unwrap())))
            },
            Self::Unix(_)=>unimplemented!()                
        }
    }
    pub async  fn poll(&self)->NetPollState {
        match self {
            Self::Tcp(tcp)=>unimplemented!(),
            Self::Udp(udp)=>udp.poll().await,
            Self::Unix(_)=>unimplemented!(),
        }
    }

    pub fn shutdown(&self,method:u8)->Result<(),SysError> {
        match self {
            Self::Tcp(tcp)=>tcp.shutdown(method),
            Self::Udp(udp)=>udp.shutdown(),
            Self::Unix(_)=>unimplemented!()
        }
    }
}

pub struct Socket{
    // The type of socket (such as STREAM, DGRAM
    pub types:SocketType,
    //// The core of a socket, which includes TCP, UDP, or Unix domain sockets
    pub sk:Sock,//facing to kernel
    // File metadata, including metadata information related to sockets
    pub file:File
}

unsafe impl Sync for Socket {
    
}
unsafe impl Send for Socket {
    
}

impl Socket {
    pub fn new(domain:SaFamily,sockettype:SocketType,nonblock:bool)->Self{
        let sk=match domain {
            SaFamily::AF_Inet|SaFamily::AF_Inet6=>{
                match sockettype {
                    SocketType::STREAM=>Sock::Tcp(TcpSocket::new()),
                    SocketType::DGRAM=>Sock::Udp(UdpSocket::new()),
                    _=>unimplemented!()                    
                }
            },
            SaFamily::AF_Unix=>unimplemented!(),
        };
        let flag=if nonblock {
            sk.set_nonblocking();
            O_RDWR|O_NONBLOCK
        }else{
            O_RDWR
        };
        //here the path is zero_init inode is zero_init
        Socket { types: sockettype, sk: sk, file:File::new(Path::zero_init(),Dentry::zero_init().get_inode() , flag)} 
    }
    // pub fn alloc_fd(&self)->usize {
    //     let fd=current_task().alloc_fd(self);
    //     self.fd=Some(fd);
    //     fd
    // }

    pub fn from_another(another:&Self,sk:Sock)->Self{
        Self { 
            types: another.types, sk: sk, file:File::new(Path::zero_init(), Dentry::zero_init().get_inode() , O_RDWR)  
        }
    }
}



impl FileOp for Socket {
    fn as_any(&self) -> &dyn core::any::Any {
        todo!()
    }

    fn read<'a>(&'a self, buf: &'a mut [u8]) -> usize {
        if buf.len()==0 {
            return 0;
        }
        warn!(
            "[Socket::File::read_at] expect to recv: {:?}",
            buf.len()
        );
        block_on(self.sk.recvfrom(buf)).unwrap().0
    }

    fn write<'a>(&'a self, buf: &'a [u8]) -> usize {
        if buf.len()==0 {
            return 0;
        }
            warn!(
                "[Socket::File::read_at] expect to recv: {:?}",
                buf.len()
            );
            block_on(self.sk.sendto(buf,None)).unwrap()
    }

    fn seek(&self, offset: usize) {
        todo!()
    }

    fn get_offset(&self) -> usize {
        todo!()
    }

    fn readable(&self) -> bool {
        todo!()
    }

    fn writable(&self) -> bool {
        todo!()
    }
}
    
//  impl async_FileOp for Socket {
//     async fn read<'a>(&'a self, buf: &'a mut [u8]) -> usize {
//         if buf.len()==0 {
//             return 0;
//         }
//         let (len,addr)=self.sk.recvfrom(buf).await.unwrap(); 
//         warn!(
//             "[Socket::File::read_at] expect to recv: {:?} exact: {len}",
//             buf.len()
//         );
//         len
//     }
//     async fn write<'a>(&'a self, buf: &'a [u8]) -> usize {
//         if buf.len()==0 {
//             return 0;
//         }
//         let len=self.sk.sendto(buf, None).await.unwrap();
//         warn!(
//             "[Socket::File::write_at] expect to send: {:?} bytes exact: {len}",
//             buf.len()
//         );
//         len
//     }
//  }
impl DowncastArc for Socket {
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any> {
        return self;
    }
}



