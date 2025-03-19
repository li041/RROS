/*
 * @Author: Peter/peterluck2021@163.com
 * @Date: 2025-03-17 22:26:39
 * @LastEditors: Peter/peterluck2021@163.com
 * @LastEditTime: 2025-03-19 15:54:24
 * @FilePath: /os/src/net/addr.rs
 * @Description: net addr display and transfer from each other
 * 
 * Copyright (c) 2025 by peterluck2021@163.com, All Rights Reserved. 
 */
use alloc::{fmt, format, string::String};
use net::{IpAddress, IpEndpoint, IpListenEndpoint, Ipv4Address, Ipv6Address};

use super::SaFamily;



#[derive(Clone, Copy)]
//ipv4 address
pub struct SockAddrIn{
    //domain for ipv4 is ipnet
    pub family:u16,
    // port in network byte order
    pub port:[u8;2],
    /// address in network byte order
    pub addr:[u8;4],
    pub zero:[u8;8],
}



//display
impl fmt::Display for SockAddrIn {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let port=u16::from_be_bytes(self.port);
        let addr=format!(
            "{}.{}.{}.{}",
            self.addr[0],self.addr[1],self.addr[2],self.addr[3]
        );
        write!(f,"AF_NET:{}:{}",addr,port)
    }
}

#[derive(Clone, Copy)]
//ipv6 net
pub struct SockAddrIn6{
    pub family:u16,
    pub addr:[u8;16],
    pub port:[u8;2],
    pub flowinfo: u32,
    pub scope: u32,
}

impl fmt::Display for SockAddrIn6 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let port=u16::from_be_bytes(self.port);
        let addr=format!(
            "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
            u16::from_be_bytes([self.addr[0], self.addr[1]]),
            u16::from_be_bytes([self.addr[2], self.addr[3]]),
            u16::from_be_bytes([self.addr[4], self.addr[5]]),
            u16::from_be_bytes([self.addr[6], self.addr[7]]),
            u16::from_be_bytes([self.addr[8], self.addr[9]]),
            u16::from_be_bytes([self.addr[10], self.addr[11]]),
            u16::from_be_bytes([self.addr[12], self.addr[13]]),
            u16::from_be_bytes([self.addr[14], self.addr[15]])
        );
        write!(f,"AF_NET6:{}:{}",addr,port)
    }
}

#[derive(Clone, Copy)]
pub struct SockAddrUn{
    pub family:u16,
    pub path:[u8;100],
}


impl fmt::Display for SockAddrUn {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let path = match self.path.iter().position(|&x| x == 0) {
            Some(pos) => String::from_utf8_lossy(&self.path[..pos]),
            None => String::from_utf8_lossy(&self.path),
        };
        write!(f,"AF_UNIX:{}",path)
    }
}

pub union SocketAddr {
    pub family:u16,
    pub ipv4:SockAddrIn,
    pub ipv6:SockAddrIn6,
    pub unix:SockAddrUn,
}

impl fmt::Display for SocketAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unsafe {
        match self.family {
            1=>write!(f,"{}",self.unix),
            2=>write!(f,"{}",self.ipv4),
            10=>write!(f,"{}",self.ipv6),
            _=>write!(f,"unkown addr family")
        }}
    }
}

impl SocketAddr {
    pub fn into_endpoint(&self)->IpEndpoint {
        unsafe {
            match SaFamily::try_from(self.family).unwrap() {
            SaFamily::AF_Inet=>{
                IpEndpoint::new(IpAddress::Ipv4(Ipv4Address(self.ipv4.addr)), u16::from_be_bytes(self.ipv4.port))
            },
            SaFamily::AF_Unix=>{
                panic!("Shouldn't get there")
            },
            SaFamily::AF_Inet6=>{
                IpEndpoint::new(IpAddress::Ipv6(Ipv6Address(self.ipv6.addr)), u16::from_be_bytes(self.ipv6.port))
            } 
            }
         }
    }
    pub fn into_listen_endpoint(&self)->IpListenEndpoint{
        match SaFamily::try_from(unsafe { self.family }).unwrap() {
            SaFamily::AF_Inet=>{
                unsafe { self.ipv4.into() }
            },
            SaFamily::AF_Inet6=>{unsafe { self.ipv6.into() }},
            SaFamily::AF_Unix=>panic!("Shouldn't get there"),
        }
    }
    pub fn from_endpoint(endpoint:IpEndpoint)->Self{
        match endpoint.addr {
            IpAddress::Ipv4(ip)=>SocketAddr{
                    ipv4:endpoint.into()
            },
            IpAddress::Ipv6(address) => SocketAddr{
                    ipv6:endpoint.into()
            },
        }
    }
    
}

impl From<SockAddrIn> for IpListenEndpoint {
    fn from(value: SockAddrIn) -> Self {
        IpListenEndpoint{
            addr:Some(IpAddress::Ipv4(Ipv4Address(value.addr))),
            port:u16::from_be_bytes(value.port)
        }
    }
}
impl From<SockAddrIn6> for IpListenEndpoint {
    fn from(value: SockAddrIn6) -> Self {
        IpListenEndpoint{
            addr:Some(IpAddress::Ipv6(Ipv6Address(value.addr))),
            port:u16::from_be_bytes(value.port)
        }
    }
}
impl From<SockAddrIn> for IpEndpoint {
    fn from(value: SockAddrIn) -> Self {
        IpEndpoint::new(IpAddress::Ipv4(Ipv4Address(value.addr)), u16::from_be_bytes(value.port))
    }
}
impl  From<SockAddrIn6> for IpEndpoint {
    fn from(value: SockAddrIn6) -> Self {
        IpEndpoint::new(IpAddress::Ipv6(Ipv6Address(value.addr)), u16::from_be_bytes(value.port))
    }
}

impl Into<SockAddrIn> for IpEndpoint {
    fn into(self) -> SockAddrIn {
        if let IpAddress::Ipv4(ip) = self.addr {
            SockAddrIn{
                family:2,
                port:self.port.to_be_bytes(),
                addr:unsafe {
                    core::mem::transmute::<Ipv4Address,[u8;4]>(ip)
                },
                zero:[0;8]
            }
        }
        else {
            panic!();
        }
    }
}

impl  Into<SockAddrIn6> for IpEndpoint {
    fn into(self) -> SockAddrIn6 {
        if let IpAddress::Ipv6(ip) = self.addr {
            SockAddrIn6{
                addr:unsafe {
                    core::mem::transmute::<Ipv6Address,[u8;16]>(ip)
                },
                family: SaFamily::AF_Inet6.into(),
                port: self.port.to_be_bytes(),
                flowinfo: 0,
                scope: 0,
                
            }
        }
        else {
            panic!();
        }
    }
}
  