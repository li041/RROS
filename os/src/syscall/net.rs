/*
 * @Author: peter
 * @Date: 2025-03-06 22:58:38
 * @LastEditors: peter
 * @LastEditTime: 2025-03-07 00:05:44
 * @FilePath: /RROS/os/src/syscall/net.rs
 * @Description: 
 * 
 * Copyright (c) 2025 by ${git_name_email}, All Rights Reserved. 
 */

use crate::net;

 ///sys_socket create a socket on domain which define the socket address,types which define the socket type and the protocol which define which protocol i should use
 /// return fd alloc by fileallocater
pub fn sys_socket(domain:usize,types:i32,protocol:usize){
    
}