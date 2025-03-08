/*
 * @Author: peter
 * @Date: 2025-03-08 22:05:44
 * @LastEditors: peter
 * @LastEditTime: 2025-03-08 22:34:44
 * @FilePath: /net/src/addr.rs
 * @Description: convert addr between smoltcp and core
 * 
 * Copyright (c) 2025 by ${git_name_email}, All Rights Reserved. 
 */


 use smoltcp::wire::{IpEndpoint,IpListenEndpoint,IpAddress,Ipv6Address};

 pub fn to_ipendpoint(iplistenendpoint:IpListenEndpoint)->IpEndpoint{
    let ip=iplistenendpoint.addr.unwrap();
    IpEndpoint::new(ip, iplistenendpoint.port)
 }

//定义未初始化ipv4,ipv6地址与端口
pub const UNSPECIFIED_IPV4: IpAddress=IpAddress::v4(0, 0,0, 0);
pub const UNSPECIFIED_IPV4_ENDPOINT:IpEndpoint=IpEndpoint::new(UNSPECIFIED_IPV4, 0);
pub const UNSPECIFIED_IPV6:IpAddress=IpAddress::Ipv6(Ipv6Address::UNSPECIFIED);
pub const UNSPECIFIED_IPV6_ENDPOINT:IpEndpoint=IpEndpoint::new(UNSPECIFIED_IPV6, 0);
//定义本地ipv4,ipv6地址端口
pub const LOCAL_IPV4:IpAddress=IpAddress::v4(127, 0, 0, 0);
pub const LOCAL_IPV4_ENDPOINT:IpEndpoint=IpEndpoint::new(LOCAL_IPV4, 0);


 