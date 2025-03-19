use log::info;
/*
 * @Author: Peter/peterluck2021@163.com
 * @Date: 2025-03-18 22:35:22
 * @LastEditors: Peter/peterluck2021@163.com
 * @LastEditTime: 2025-03-19 17:29:38
 * @FilePath: /os/src/syscall/net.rs
 * @Description: net syscall
 * 
 * Copyright (c) 2025 by peterluck2021@163.com, All Rights Reserved. 
 */
use net::{addr, err::SysError};
use virtio_drivers::device::socket;
use alloc::{sync::Arc, task, vec::Vec};
use crate::{net::{addr::SocketAddr, socket::{Sock, Socket}, SaFamily, SocketType}, task::current_task};


///sys_socket create a socket and alloc a fd for socket.the fd will be the lowest numbered fd
pub fn sys_socket(domin:usize,types:u16,protocal:usize)->Result<usize,SysError> {
    //get domain for socket
    let domain=SaFamily::try_from(domin as u16)?;
    let nonblock=false;

    //todo?
    let types=SocketType::try_from(types)?;

    let socket=Socket::new(domain, types, nonblock);
    let task=current_task();
    //alloc fd for socket
    let fd=task.fd_table().alloc_fd(Arc::new(socket));
    log::info!("[sys_socket] new socket {domain:?} {types:?} in fd {fd}, nonblock:{nonblock}");
    Ok(fd)
}
/// When a socket is created with socket(2), it exists in a name space
/// (address family) but has no address assigned to it.  bind() assigns the
/// address specified by addr to the socket referred to by the file
/// descriptor sockfd.  addrlen specifies the size, in  bytes,  of the
/// address structure pointed to by addr.  Traditionally, this operation is
/// called “assigning a name to a socket”.
///bind socket described by sockfd to addr,write bind addr to local addr
pub fn sys_bind(sockfd:usize,addr:usize,addrlen:usize)->Result<usize,SysError> {
    let task=current_task();
    let local_addr=task.read_sockaddr(addr, addrlen)?;
    let socket=task.sockfd_lookup(sockfd).unwrap();
    socket.sk.bind(sockfd, local_addr)?;
    Ok(0)
}

pub fn sys_listen(sockfd:usize,backlog:usize)->Result<usize,SysError> {
    let task=current_task();
    let socket=task.sockfd_lookup(sockfd)?;
    socket.sk.listen();
    Ok(0)
}
/// Connect the active socket referenced by the file descriptor `sockfd` to
/// the listening socket specified by `addr` and `addrlen` at the address
pub fn sys_connect(sockfd:usize,addr:usize,addrlen:usize)->Result<usize,SysError> {
    let task=current_task();
    let socket=task.sockfd_lookup(sockfd)?;
    let remote_addr=task.read_sockaddr(addr, addrlen)?;
    
    log::info!("[sys_connect] sockfd{sockfd} are connect to {remote_addr}");
    socket.sk.connect(remote_addr);
    Ok(0)
}

    /// The accept() system call accepts an incoming connection on a listening
    /// stream socket referred to by the file descriptor `sockfd`. If there are
    /// no pending connections at the time of the accept() call, the call
    /// will block until a connection request arrives. Both `addr` and
    /// `addrlen` are pointers representing peer socket address. if the addrlen
    /// pointer is not zero, it will be assigned to the actual size of the
    /// peer address.
    ///
    /// On success, the call returns the file descriptor of the newly connected
    /// socket.
pub async fn sys_accept(sockfd:usize,addr:usize,addrlen:usize)->Result<usize,SysError> {
    let task=current_task();
    let socket=task.sockfd_lookup(sockfd)?;
    task.set_blocked();
    // task.set_wake_up_signal(!*task.sig_mask_ref());
    log::info!("[sys_accept]:waiting for accept...");
    let new_sk=socket.sk.accept().await?;
    task.set_running();
    log::info!("[sys_accept]:has accepted...");
    let peer_addr=new_sk.peer_addr()?;
    let peer_sockaddr=SocketAddr::from_endpoint(peer_addr);
    log::info!("[sys_accept]:accpet peer addr {peer_sockaddr}");
    // todo!();
    task.write_sockaddr(addr, addrlen, peer_sockaddr);
    let newsocket=Arc::new(Socket::from_another(&socket, Sock::Tcp(new_sk)));
    let fd=task.fd_table().alloc_fd(newsocket);
    Ok(fd)
}

//return socket named by sockfd and write the local_addr to addr passes in functions
pub fn sys_getsockname(sockfd:usize,addr:usize,addrlen:usize)->Result<usize,SysError> {
    let task=current_task();
    let socket=task.sockfd_lookup(sockfd)?;
    let local_addr=socket.sk.local_addr()?;
    log::info!("[sys_getsockname] local addr: {local_addr}");
    task.write_sockaddr(addr, addrlen,local_addr);
    Ok(0)
}
pub fn sys_getpeername(sockfd:usize,addr:usize,addrlen:usize)->Result<usize,SysError> {
    let task=current_task();
    let socket=task.sockfd_lookup(sockfd)?;
    let peer_addr=socket.sk.peer_addr()?;
    task.write_sockaddr(addr, addrlen, peer_addr);
    Ok(0)
}
    /// Usually used for sending UDP datagrams. If using `sys_sendto` for STEAM,
    /// `dest_addr` and `addrlen` will be ignored.
    ///
pub async fn sys_sendto(sockfd:usize,buf:&[u8],len:usize,flags:usize,dest_addr:usize,dest_addrlen:usize)->Result<usize,SysError> {
    let task=current_task();
    let socket=task.sockfd_lookup(sockfd)?;
    task.set_blocked();
    let bytes=match socket.types {
        SocketType::STREAM => {
            if dest_addr!=0 {
                return Err(SysError::ENOTCONN);
            }
            socket.sk.sendto(buf, None).await?
        },
        SocketType::DGRAM =>{
            let remote_addr=task.read_sockaddr(dest_addr, dest_addrlen)?;
            socket.sk.sendto(buf, Some(remote_addr)).await?
        },
        _=>unimplemented!()
    };
    task.set_running();
    Ok(bytes)    
}
    /// - `sockfd`: Socket descriptor, created through socket system calls.
    /// - `buf`: A pointer to a buffer used to store received data.
    /// - `len`: The length of the buffer, which is the maximum number of data
    ///   bytes received.
    /// - `flags`: Currently ignored
    /// - `src_addr`: A pointer to the sockaddr structure used to store the
    ///   sender's address information. Can be `NULL`, if the sender address is
    ///   notrequired.
    /// - `src_adddrlen`: A pointer to the socklen_t variable, used to store the
    ///   size of src_addr. When calling, it should be set to the size of the
    ///   src_addr structure, which will include the actual address size after
    ///   the call. Can be `NULL`, if src_addr is `NULL`.
    ///
    /// Return the number of bytes received
/// recvfrom src_addr which be pointed at src_addr
pub async fn sys_recvfrom(sockfd:usize,buf:&mut [u8],len:usize,src_addr:usize,src_addrlen:usize)->Result<usize,SysError> {
    let task=current_task();
    let socket=task.sockfd_lookup(sockfd)?;
    let mut temp=Vec::with_capacity(len);
    unsafe {temp.set_len(len);}
    task.set_blocked();
    info!("[sys_recvfrom] sockfd{sockfd} local addr:{}  is trying to recvfrom addr {}",socket.sk.local_addr().unwrap(),socket.sk.peer_addr().unwrap());
    let (bytes,remote_addr)=socket.sk.recvfrom(&mut temp).await?;
    task.set_running();
    task.write_sockaddr(src_addr, src_addrlen, remote_addr);
    if bytes > 0 {
        let copy_len = buf.len().min(bytes); // 取两者较小值
        buf[..copy_len].copy_from_slice(&temp[..copy_len]);
    }
    Ok(bytes)
}

pub fn sys_shutdown(sockfd:usize,method:usize)->Result<usize,SysError>{
    let task=current_task();
    let socket=task.sockfd_lookup(sockfd)?;
    log::info!(
        "[sys_shutdown] sockfd:{sockfd} shutdown {}",
        match method {
            0 => "READ",
            1 => "WRITE",
            2 => "READ AND WRITE",
            _ => "Invalid argument",
        }
    );
    socket.sk.shutdown(method as u8);
    Ok(0)
}

