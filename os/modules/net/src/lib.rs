/*
 * @Author: peter
 * @Date: 2025-03-08 21:49:57
 * @LastEditors: peter
 * @LastEditTime: 2025-03-09 17:15:55
 * @FilePath: /net/src/lib.rs
 * @Description:modules for net,package the device for netdevice
 * 
 * Copyright (c) 2025 by ${git_name_email}, All Rights Reserved. 
 */
#![no_main]
#![no_std]

extern crate alloc;
mod listentable;
mod addr;

use core::cell::RefCell;

use alloc::slice;
use alloc::{vec,boxed::Box};
use log::log;
use smoltcp::wire::{EthernetAddress, HardwareAddress};
use spin::mutex::Mutex;
use spin::Mutex;
use smoltcp::iface::{Config, Interface, SocketHandle, SocketSet};
use smoltcp::socket::{self, AnySocket};
use smoltcp::phy::{Device, Medium, RxToken, TxToken};
use netdevice::{self, Netdevice, NetdeviceBuffer};

const TCP_SEND_BUFFER:usize=64*1024;
const TCP_RECV_BUFFER:usize=64*1024;
const UDP_SEND_BUFFER:usize=64*1024;
const UDP_RECV_BUFFER:usize=64*1024;
const RANDOM_SEED: u64 = 0xA2CE_05A2_CE05_A2CE;

///将somltcp中的socketset封装到socketsetwrapper中，控制所有socket somltcp
struct SocketSetWrapper<'a>(Mutex<SocketSet<'a>>);

impl<'a> SocketSetWrapper<'a> {
    pub fn new()->Self{
        SocketSetWrapper(Mutex::new(SocketSet::new(vec![])))
    }

    pub fn new_tcp_socket()->socket::tcp::Socket{
        let tcp_send_buffer=socket::tcp::SocketBuffer::new(vec![0;TCP_SEND_BUFFER]);
        let tcp_recv_buffer=socket::tcp::SocketBuffer::new(vec![0;TCP_RECV_BUFFER]);
        socket::tcp::Socket::new(tcp_recv_buffer, tcp_send_buffer)
    }
    pub fn new_udp_socket()->socket::udp::Socket{
        let udp_send_buffer=socket::udp::PacketBuffer::new(vec![socket::udp::PacketMetadata;8], vec![0;UDP_SEND_BUFFER]);
        let udp_recv_buffer=socket::udp::PacketBuffer::new(vec![socket::udp::PacketBuffer;8], vec![0;UDP_RECV_BUFFER]);
        socket::udp::Socket::new(udp_recv_buffer, udp_send_buffer)
    }
    pub fn add(&self,socket:AnySocket)->SocketHandle{
        self.0.lock().add(socket)
    }
    pub fn with_socket<F,R,T:AnySocket>(&self,sockethandle:SocketHandle,f:F)->R
    where F:FnOnce(&T)->R
    {
        let socketset=self.0.lock();
        let socket=socketset.get(sockethandle);
        f(socket)
    }
    pub fn with_mut_socket<T:AnySocket,F,R>(&self,sockethandle:SocketHandle)->R
    where F:FnOnce(&mut F)->R
    {
        let socket=self.0.lock().get_mut(sockethandle);
        f(socket)
    }
    pub fn remove(&self,handle:SocketHandle){
        self.0.lock().remove(handle);
        log!("remove socket{}",SocketHandle);
    }
}

//封装device网卡，网卡要求实现在module device中定义的trait
struct DeviceWrapper{
    inner:RefCell<Box<dyn Netdevice>>
}
impl DeviceWrapper {
    pub fn new(innner:RefCell<Box<dyn Netdevice>>)->Self{
        DeviceWrapper{
            inner:innner
        }
    }
}

//实现trait中功能
impl Device for DeviceWrapper {
    type RxToken<'a>=NetRxToken;

    type TxToken<'a>=NetTxToken;
    /// Construct a token pair consisting of one receive token and one transmit
    /// token.
    ///
    /// The additional transmit token makes it possible to generate a reply
    /// packet based on the contents of the received packet. For example,
    /// this makes it possible to handle arbitrarily large ICMP echo
    /// ("ping") requests, where the all received bytes need to be sent
    /// back, without heap allocation.
    ///
    /// The timestamp must be a number of milliseconds, monotonically increasing
    /// since an arbitrary moment in time, such as system startup.
    ///
    /// 构造一个由一个接收令牌和一个发送令牌组成的令牌对。
    /// 附加的传输令牌使得可以基于接收到的数据包的内容生成回复数据包。例如，
    /// 这使得处理任意大的ICMP回显（“ping”）请求成为可能，
    /// 其中所有接收到的字节都需要被发回，而无需堆分配。
    fn receive(&mut self, timestamp: smoltcp::time::Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let mut device=self.inner.borrow_mut();
        //如果无法发送，则recv之后无法返回socket
        if !device.isok_send() {
            return None;
        }

        let recv_buf=match device.recv() {
            Ok(buf)=>buf,
            _=>None,
        };
        Some((NetRxToken(recv_buf,&self.inner),NetTxToken(&self.inner)))
    }
    /// Construct a transmit token.
    ///
    /// The timestamp must be a number of milliseconds, monotonically increasing
    /// since an arbitrary moment in time, such as system startup.
    fn transmit(&mut self, timestamp: smoltcp::time::Instant) -> Option<Self::TxToken<'_>> {
        let device=self.inner.borrow_mut();
        if !device.isok_send(){
            return  None;
        }
        Some(NetTxToken(self.inner.borrow()))
    }
    ///返回网卡对于的数据信息
    fn capabilities(&self) -> smoltcp::phy::DeviceCapabilities {
        self.inner.borrow().capabilities()
    }
}


struct NetRxToken{
    recv_buffer:Box<dyn NetdeviceBuffer>,
    device:RefCell<Box<dyn Netdevice>>
}
impl RxToken for NetRxToken {
    //consume a token to receive a package ,then call the f function on the mutable buffer
    /// Consumes the token to receive a single network packet.
    ///
    /// This method receives a packet and then calls the given closure `f` with
    /// the raw packet bytes as argument.
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R {
            let mut device=self.device.borrow_mut();
            let buf=self.recv_buffer.package_mut();
            let ret=f(buf);
            //回收package
            device.recycle_recv_buffer(buf);
            ret
    }
}
struct NetTxToken{
    device:RefCell<Box<dyn Netdevice>>
}
impl TxToken for NetTxToken {
     /// This method constructs a transmit buffer of size `len` and calls the
    /// passed closure `f` with a mutable reference to that buffer. The
    /// closure should construct a valid network packet (e.g. an ethernet
    /// packet) in the buffer. When the closure returns, the transmit buffer
    /// is sent out.
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R {
            let buf=self.device.borrow_mut().alloc_send_buffer(size).unwrap();
            let ret=f(buf.package_mut());//create a send socket package on this buf,now we can send 
            self.device.send(buf);
            ret
    }
}

struct InterfaceWrapper{
    name:&str,
    eth_address:EthernetAddress,
    device:Mutex<DeviceWrapper>,
    ifce:Mutex<Interface>
}

impl InterfaceWrapper {
    pub fn new(name:&str,eth_address:EthernetAddress,device:Box<dyn Netdevice>)->Self{
        let config: Config=match device.capabilities() {
            Medium::Ethernet=>Config::new(HardwareAddress::Ethernet(eth_address)),
            Medium::Ip=>Config::new(HardwareAddress::Ip)
        };
    /// Random seed.
    ///
    /// It is strongly recommended that the random seed is different on each
    /// boot, to avoid problems with TCP port/sequence collisions.
        config.random_seed=RANDOM_SEED;


        // InterfaceWrapper{
        //     name:name,
        //     eth_address:eth_address,
        //     device:Mutex<device>,
        //     ifce:
        // }
    }
}





