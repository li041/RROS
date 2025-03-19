/*
 * @Author: Peter/peterluck2021@163.com
 * @Date: 2025-03-11 16:59:38
 * @LastEditors: Peter/peterluck2021@163.com
 * @LastEditTime: 2025-03-17 20:38:52
 * @FilePath: /modules/net/src/addr.rs
 * @Description: 
 * 
 * Copyright (c) 2025 by peterluck2021@163.com, All Rights Reserved. 
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

pub const UNSPECIFIED_LISTEN_ENDPOINT:IpListenEndpoint=IpListenEndpoint{
   addr:None,
   port:0
};