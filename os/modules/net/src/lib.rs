/*
 * @Author: peter
 * @Date: 2025-03-08 21:49:57
 * @LastEditors: Peter/peterluck2021@163.com
 * @LastEditTime: 2025-03-19 20:49:37
 * @FilePath: /os/modules/net/src/lib.rs
 * @Description:modules for net,package the device for netdevice
 * 
 * Copyright (c) 2025 by ${git_name_email}, All Rights Reserved. 
 */
#![no_main]
#![no_std]

extern crate alloc;
extern crate lazy_static;

pub mod listentable;
pub mod addr;
pub mod bench;
pub mod portmap;
pub mod udp;
pub mod err;
pub mod tcp;
// pub mod SyncFlag;
use alloc::vec::Vec;
use listentable::LISTEN_TABLE;
use log::warn;
use core::cell::RefCell;
use core::ops::DerefMut;
use lazy_static::lazy_static;
// use alloc::slice;
use alloc::{vec,boxed::Box};
use log::{error, info};
use smoltcp::time::{Duration, Instant};
pub use smoltcp::wire::{IpEndpoint,IpListenEndpoint,Ipv4Address,Ipv6Address,EthernetAddress, HardwareAddress, IpAddress, IpCidr};
use spin::mutex::Mutex;
use spin::Once;
use smoltcp::iface::{Config, Interface, SocketHandle, SocketSet};
use smoltcp::socket::{self, AnySocket};
use smoltcp::phy::{Device, Medium, RxToken, TxToken};
use netdevice::{self, DevError, Netdevice, NetdeviceBuffer};
use riscv::register::time;


const TCP_SEND_BUFFER:usize=64*1024;
const TCP_RECV_BUFFER:usize=64*1024;
const UDP_SEND_BUFFER:usize=64*1024;
const UDP_RECV_BUFFER:usize=64*1024;
const RANDOM_SEED: u64 = 0xA2CE_05A2_CE05_A2CE;
const STANDARD_MTU: usize = 1500;
const IP_PREFIX:u8=24;
lazy_static!(
    static ref SOCKET_SET: SocketSetWrapper<'static> = SocketSetWrapper::new();
    static ref ETH0:Once<InterfaceWrapper<'static>>=Once::new();
);

macro_rules! env_or_default {
    ($key:literal) => {
        match option_env!($key) {
            Some(val) => val,
            None => "",
        }
    };
}

/// Defined in makefile
const IP: &str = env_or_default!("RROS_IP");
const GATEWAY: &str = env_or_default!("RROS_GW");

// static SOCKET_SET: SocketSetWrapper = SocketSetWrapper::new();
// static ETH0:Once<InterfaceWrapper>=Once::new();

///将somltcp中的socketset封装到socketsetwrapper中，控制所有socket somltcp
struct SocketSetWrapper<'a>(Mutex<SocketSet<'a>>);

impl<'a> SocketSetWrapper<'a> {
    pub fn new()->Self{
        SocketSetWrapper(Mutex::new(SocketSet::new(vec![])))
    }

    pub fn new_tcp_socket()->socket::tcp::Socket<'a>{
        let tcp_send_buffer=socket::tcp::SocketBuffer::new(vec![0;TCP_SEND_BUFFER]);
        let tcp_recv_buffer=socket::tcp::SocketBuffer::new(vec![0;TCP_RECV_BUFFER]);
        socket::tcp::Socket::new(tcp_recv_buffer, tcp_send_buffer)
    }
    pub fn new_udp_socket()->socket::udp::Socket<'a>{
        let udp_send_buffer=socket::udp::PacketBuffer::new(vec![socket::udp::PacketMetadata::EMPTY;8], vec![0;UDP_SEND_BUFFER]);
        let udp_recv_buffer=socket::udp::PacketBuffer::new(vec![socket::udp::PacketMetadata::EMPTY;8], vec![0;UDP_RECV_BUFFER]);
        socket::udp::Socket::new(udp_recv_buffer, udp_send_buffer)
    }
    pub fn add<T: AnySocket<'a>>(&self,socket:T)->SocketHandle{
        self.0.lock().add(socket)
    }
    pub fn with_socket<T:AnySocket<'a>,F,R>(&self,sockethandle:SocketHandle,f:F)->R
    where F:FnOnce(&T)->R
    {
        let socketset=self.0.lock();
        let socket=socketset.get(sockethandle);
        f(socket)
    }
    pub fn with_mut_socket<T:AnySocket<'a>,F,R>(&self,sockethandle:SocketHandle,f:F)->R
    where F:FnOnce(&mut T)->R
    {
        let mut set = self.0.lock();
        let socket =set.get_mut(sockethandle);
        f(socket)
    }
    pub fn remove(&self,handle:SocketHandle){
        self.0.lock().remove(handle);
        info!("remove socket{}",handle);
    }
    pub fn poll_interface(&self)->Instant{
        ETH0.get().unwrap().poll(&mut &self.0)
    }
    pub fn check_poll(&self,time:Instant){
        ETH0.get().unwrap().check_poll(time, &self.0);
    }
}

//封装device网卡，网卡要求实现在module device中定义的trait
struct DeviceWrapper{
    inner:RefCell<Box<dyn Netdevice>>
}
impl DeviceWrapper {
    pub fn new(inner:RefCell<Box<dyn Netdevice>>)->Self{
        DeviceWrapper{
            inner:inner
        }
    }
}

//实现trait中功能
impl Device for DeviceWrapper {
    type RxToken<'a>=NetRxToken<'a>;

    type TxToken<'a>=NetTxToken<'a>;
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
    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let mut device=self.inner.borrow_mut();

        ////????为什么要回收sendbuffer
        if let Err(e) =  device.recycle_send_buffer(){
            warn!("recycle_tx_buffers failed: {:?}", e);
            return None;
        }
        //如果无法发送，则recv之后无法返回socket
        if !device.isok_send() {
            return None;
        }
        let recv_buf=match device.recv() {
            Ok(buf)=>buf,
            Err(err)=>{
                if !matches!(err, DevError::Again) {
                    warn!("receive failed: {:?}", err);
                }
                return None;
            }
        };
        
        Some((NetRxToken{recv_buffer:recv_buf,device:&self.inner},NetTxToken{device:&self.inner}))
    }
    /// Construct a transmit token.
    ///
    /// The timestamp must be a number of milliseconds, monotonically increasing
    /// since an arbitrary moment in time, such as system startup.
    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        //由Totxtoken来send,alloc buffer for send
        let mut device=self.inner.borrow_mut();
        if let Err(e) = device.recycle_send_buffer() {
            error!("{:?}", e);
        }
        if !device.isok_send(){
            return  None;
        }
        Some(NetTxToken{device:&self.inner})
    }
    ///返回网卡对于的数据信息
    fn capabilities(&self) -> smoltcp::phy::DeviceCapabilities {
        self.inner.borrow().capabilities()
    }
}


struct NetRxToken<'a>{
    recv_buffer:Box<dyn NetdeviceBuffer>,//recv buf for data
    device:&'a RefCell<Box<dyn Netdevice>>//device to rece and send ,alloc,recycel
}
impl<'a> RxToken for NetRxToken<'a> {
    //consume a token to receive a package ,then call the f function on the mutable buffer
    /// Consumes the token to receive a single network packet.
    ///
    /// This method receives a packet and then calls the given closure `f` with
    /// the raw packet bytes as argument.
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R {
            let mut device=self.device.borrow_mut();
            let mut buf=self.recv_buffer;
            let ret=f(buf.package_mut());
            //回收package
            device.recycle_recv_buffer(buf);
            ret
    }
    
    fn preprocess(&self, sockets: &mut SocketSet<'_>) {
        let medium = self.device.borrow().capabilities().medium;
        let is_ethernet = medium == Medium::Ethernet;
        snoop_tcp_packet(self.recv_buffer.package(), sockets, is_ethernet).ok();
    }
    
    fn meta(&self) -> smoltcp::phy::PacketMeta {
        smoltcp::phy::PacketMeta::default()
    }
}
struct NetTxToken<'a>{
    device:&'a RefCell<Box<dyn Netdevice>>
}
impl<'a> TxToken for NetTxToken<'a> {
     /// This method constructs a transmit buffer of size `len` and calls the
    /// passed closure `f` with a mutable reference to that buffer. The
    /// closure should construct a valid network packet (e.g. an ethernet
    /// packet) in the buffer. When the closure returns, the transmit buffer
    /// is sent out.
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R {
            let mut buf=self.device.borrow_mut().alloc_send_buffer(len).unwrap();
            let ret=f(buf.package_mut());//create a send socket package on this buf,now we can send 
            self.device.borrow_mut().send(buf);
            ret
    }
    
    // fn set_meta(&mut self, meta: smoltcp::phy::PacketMeta) {

    // }
}

struct InterfaceWrapper<'a>{
    name:&'a str,
    eth_address:EthernetAddress,
    device:Mutex<DeviceWrapper>,
    iface:Mutex<Interface>
}

impl<'a> InterfaceWrapper<'a> {
    pub fn new(name:&'static str,eth_address:EthernetAddress,device:Box<dyn Netdevice>)->Self{
        let mut config: Config=match device.capabilities().medium {
            Medium::Ethernet=>Config::new(HardwareAddress::Ethernet(eth_address)),
            Medium::Ip=>Config::new(HardwareAddress::Ip)
        };
    // Random seed.
    //
    // It is strongly recommended that the random seed is different on each
    // boot, to avoid problems with TCP port/sequence collisions.
        config.random_seed=RANDOM_SEED;


        // InterfaceWrapper{
        //     name:name,
        //     eth_address:eth_address,
        //     device:Mutex<device>,
        //     ifce:
        // }
        let mut dev=DeviceWrapper::new(RefCell::new(device));
        let iface=Mutex::new(Interface::new(config, &mut dev, InterfaceWrapper::current_time()));
        InterfaceWrapper { name: name, eth_address: eth_address, device: Mutex::new(dev), iface:iface }
    }
    fn current_time() -> Instant {
        Instant::from_micros_const(get_time_us() as i64)
    }

    fn ins_to_duration(instant: Instant) -> Duration {
        Duration::from_micros(instant.total_micros() as u64)
    }

    fn dur_to_duration(duration: Duration) -> Duration {
        Duration::from_micros(duration.total_micros() as u64)
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn ethernet_address(&self) -> EthernetAddress {
        self.eth_address
    }

    pub fn setup_ip_addr(&self, ips: Vec<IpCidr>) {
        let mut iface=self.iface.lock();
    //add ips vector to ip_addrs vectors
    //Update the IP addresses of the interface.
    //
    // # Panics
    // This function panics if any of the addresses are not unicast.
        iface.update_ip_addrs(|ip_addr|{ip_addr.extend(ips);});
        
    }

    pub fn setup_gateway(&self, gateway: IpAddress) {
        let mut iface = self.iface.lock();
        match gateway {
            IpAddress::Ipv4(v4) => iface.routes_mut().add_default_ipv4_route(v4).unwrap(),
            //todo
            IpAddress::Ipv6(_) => {info!("setup_gateway in ipaddress ipv6!!,not to do!");todo!();},
        };
    }
    pub fn poll(&self, sockets: &Mutex<SocketSet>) -> Instant {
        //examine socket in sockets whether be sended or received
        let mut dev = self.device.lock();
        let mut iface = self.iface.lock();
        let mut sockets = sockets.lock();
        let timestamp = Self::current_time();
    // Transmit packets queued in the given sockets, and receive packets queued
    // in the device.
    //
    // This function returns a boolean value indicating whether any packets
    // were processed or emitted, and thus, whether the readiness of any
    // socket might have changed.
        let result = iface.poll(timestamp, dev.deref_mut(), &mut sockets);
        log::warn!("[net::InterfaceWrapper::poll] does something have been changed? {result:?}");
        timestamp
    }


    ///check_poll主要用于测试在timestamp时是否可以测试sockets中socket是否发送，如果不能，则需要继续delay多久
    pub fn check_poll(&self, timestamp: Instant, sockets: &Mutex<SocketSet>) {
        let mut iface = self.iface.lock();
        let mut sockets = sockets.lock();
    // Return an _advisory wait time_ for calling [poll] the next time.
    // The [Duration] returned is the time left to wait before calling [poll]
    // next. It is harmless (but wastes energy) to call it before the
    // [Duration] has passed, and potentially harmful (impacting quality of
    // service) to call it after the [Duration] has passed.
    //
    // [poll]: #method.poll
    // [Duration]: struct.Duration.html
        match iface
            .poll_delay(timestamp, &mut sockets)
            .map(|dur|InterfaceWrapper::dur_to_duration(dur))
        {
            Some(Duration::ZERO) => {
                iface.poll(
                    Self::current_time(),
                    self.device.lock().deref_mut(),
                    &mut sockets,
                );
            }
            Some(delay) => {
                let next_poll = delay + InterfaceWrapper::ins_to_duration(timestamp);
                let current = get_time_duration();
                if next_poll < current {
                    iface.poll(
                        InterfaceWrapper::current_time(),
                        self.device.lock().deref_mut(),
                        &mut sockets,
                    );
                } else {
                    info!("需要设定一个定时器，什么时候继续检查");
                    todo!();
                }
            }
            None => {
                todo!();
            }
        }
    }
}

pub fn poll_interface()->Instant{
    SOCKET_SET.poll_interface()
}
pub fn check_poll(time:Instant ){
    SOCKET_SET.check_poll(time);
}

pub fn init_network(net_dev: Box<dyn Netdevice>, is_loopback: bool) {
    info!("Initialize network subsystem...");
    let ether_addr = EthernetAddress(net_dev.mac_address().0);
    let eth0 = InterfaceWrapper::new("eth0", ether_addr,net_dev);

    // let ip = IP.parse().expect("invalid IP address");

    // let gateway = GATEWAY.parse().expect("invalid gateway IP address");
    let gateway = GATEWAY.parse().unwrap();
    let ip;
    let ip_addrs = if is_loopback {
        ip = "127.0.0.1".parse().unwrap();
        vec![IpCidr::new(ip, 8)]
    } else {
        ip = "10.0.0.1".parse().unwrap();
        info!("maybe occur some error");
        info!("there need some to do");
        vec![
            IpCidr::new(IP.parse().unwrap(), 8),
            IpCidr::new(ip, IP_PREFIX),
        ]
    };
    eth0.setup_gateway(gateway);
    eth0.setup_ip_addr(ip_addrs);
    ETH0.call_once(|| eth0);
    info!("created net interface {:?}:", ETH0.get().unwrap().name());
    info!("  ether:    {}", ETH0.get().unwrap().ethernet_address());
    info!("  ip:       {}/{}", ip, 24);
    info!("  gateway:  {}", gateway);
}

pub struct NetPollState{
    recvable:bool,
    sendable:bool,
    hangup:bool
}
pub fn get_time_us() -> usize {
    time::read() / (10000000 / 1_000_000)
}

pub fn get_time_duration() -> Duration {
    Duration::from_micros(get_time_us() as u64)
}

fn snoop_tcp_packet(
    buf: &[u8],
    sockets: &mut SocketSet<'_>,
    is_ethernet: bool,
) -> Result<(), smoltcp::wire::Error> {
    use smoltcp::wire::{EthernetFrame, IpProtocol, Ipv4Packet, TcpPacket};

    // let ether_frame = EthernetFrame::new_checked(buf)?;
    // let ipv4_packet = Ipv4Packet::new_checked(ether_frame.payload())?;
    let ipv4_packet = if is_ethernet {
        let ether_frame = EthernetFrame::new_checked(buf)?;
        Ipv4Packet::new_checked(ether_frame.payload())?
    } else {
        Ipv4Packet::new_checked(buf)?
    };
    if ipv4_packet.next_header() == IpProtocol::Tcp {
        let tcp_packet = TcpPacket::new_checked(ipv4_packet.payload())?;
        let src_addr = (ipv4_packet.src_addr(), tcp_packet.src_port()).into();
        let dst_addr = (ipv4_packet.dst_addr(), tcp_packet.dst_port()).into();
        let is_first = tcp_packet.syn() && !tcp_packet.ack();
        if is_first {
            // create a socket for the first incoming TCP packet, as the later accept()
            // returns.
            info!("[snoop_tcp_packet] receive TCP");
            LISTEN_TABLE.incoming_tcp_packet(src_addr, dst_addr, sockets);
        }
    }
    Ok(())
}




    