/*
 * @Author: peter
 * @Date: 2025-03-06 22:24:45
 * @LastEditors: peter
 * @LastEditTime: 2025-03-07 00:04:33
 * @FilePath: /RROS/os/src/net/socket.rs
 * @Description: net method compilation
 * 
 * Copyright (c) 2025 by ${git_name_email}, All Rights Reserved. 
 */

use core::fmt::Error;

use systype::SysError;
enum Socketstate {
    SSFREE=0,			/* 未分配到任何文件中		*/
	SSUNCONNECTED=1,		/* 没有链接*/
	CONNECTING=2,			/* 正在链接	*/
	SSCONNECTED=3,		/* 已经链接到socket了		*/
	SSDISCONNECTING=4, //结束连接
}

///todo complete the udp tcp unix sock(sk)
enum Sock {
    Tcp(TcpSocket),
    Udp(UdpSocket),
    Unix(UnixSocket),
}

enum Socketype{
    SOCKSTREAM=1,
    SOCKDGRAM=2,
}


enum Socketdomain{
    AF_UNIX = 1,
    /// ipv4
    AF_INET = 2,
    /// ipv6
    AF_INET6 = 10,
}

impl Socketype {
    type Error= SysError;
    pub fn to_type(sockettype:i32)->Result<Self,Self::Error>{

        match sockettype{
            1=>Ok(Self::SOCKDGRAM),
            2=>Ok(Self::SOCKSTREAM),
            _=>Err(Self::Error::EINVAL),
        }
    }
}



// struct socket {
// 	socket_state		state;

// 	short			type;

// 	unsigned long		flags;

// 	struct file		*file;
// 	struct sock		*sk;
// 	const struct proto_ops	*ops; /* Might change with IPV6_ADDRFORM or MPTCP. */

// 	struct socket_wq	wq;
// };

pub struct Socket{
    state:Socketstate,
    //暂时先不考虑flag实现
    // flags:usize,
    socket_type:Socktype,
    //协议层结构体
    sock:Sock,

    //文件可以实现file trait来使用三
    // file:
    
}

impl Socket {
    fn new(domain:usize,types:i32,protocal:usize)->Self{

        Socket{
            state:Socketstate::SSFREE,
            socket_type:Socketype::to_type(sockettype),
            ///todo list
            sock:Sock::Tcp(())
        }


    }
    ///sys_socket create a socket on domain which define the socket address,types which define the socket type and the protocol which define which protocol i should use
    /// return fd alloc by fileallocater
    pub fn socket(domain: usize, types: i32, protocal: usize){
        //create a socket struct and alloc a fd with socket struct we also need to complete the file trait for socket 
    }
}


