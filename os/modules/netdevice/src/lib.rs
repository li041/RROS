/*
 * @Author: peter
 * @Date: 2025-03-08 20:33:51
 * @LastEditors: Peter/peterluck2021@163.com
 * @LastEditTime: 2025-03-17 17:51:25
 * @FilePath: /modules/netdevice/src/lib.rs
 * @Description: net device modules trait
 * 
 * Copyright (c) 2025 by ${git_name_email}, All Rights Reserved. 
 */
#![no_std]
#![no_main]
extern crate alloc;
use alloc::boxed::Box;
use core::marker::{Send,Sync};
use core::result::Result;
use smoltcp::phy::DeviceCapabilities;
pub struct DeviceMacAddress(pub [u8; 6]);
pub trait Netdevice:Send+Sync {
///#[non_exhaustive]
// pub struct DeviceCapabilities {
//     pub medium: Medium, 传输介质，以太网，光纤等
//     pub max_transmission_unit: usize,//MTU，最大数据链路层传输数据大小
//     pub max_burst_size: Option<usize>,最大突发数据大小
//     pub checksum: ChecksumCapabilities,校验和
// }
    //返回设备的使用能力
    fn capabilities(&self)->DeviceCapabilities;
    //返回设备mac地址
    fn mac_address(&self)->DeviceMacAddress;
    //是否可以发送数据
    fn isok_send(&self)->bool;
    //是否可以接受数据
    fn isok_recv(&self)->bool;
    //返回网卡最多可以存储的可发送数据包个数
    fn send_queue_size(&self)->usize;
    //返回网卡最多可以存储的可接受数据包个数
    fn recv_queue_size(&self)->usize;
    //回收已经发送结束的数据包进入发送队列，供后续使用
    fn recycle_send_buffer(&mut self)->DevResult;
    //回收以及接受结束的数据包个进入接受队列，供后续使用,recv_buf use to 存储
    fn recycle_recv_buffer(&mut self,recv_buf:Box<dyn NetdeviceBuffer>)->DevResult;
    //发送数据
    fn send(&mut self,send_buf:Box<dyn NetdeviceBuffer>)->DevResult;
    //接受数据
    fn recv(&mut self)->DevResult<Box<dyn NetdeviceBuffer>>;
    //根据发送数据大小分配一个内存用于发送加入发送队列，后续需要回收   
    fn alloc_send_buffer(&mut self,size:usize)->DevResult<Box<dyn NetdeviceBuffer>>; 
}



pub trait NetdeviceBuffer {
    //返回数据报中数据，只读
    fn package(&self)->&[u8];
    //返回数据包中数据，可写
    fn package_mut(&mut self)->&mut [u8];
    //返回数据包长度
    fn package_len(&self)->usize;
}

#[derive(Debug)]
pub enum DevError {
    /// An entity already exists.
    AlreadyExists,
    /// Try again, for non-blocking APIs.
    Again,
    /// Bad internal state.
    BadState,
    /// Invalid parameter/argument.
    InvalidParam,
    /// Input/output error.
    Io,
    /// Not enough space/cannot allocate memory (DMA).
    NoMemory,
    /// Device or resource is busy.
    ResourceBusy,
    /// This operation is unsupported or unimplemented.
    Unsupported,
}
pub type DevResult<T = ()> = Result<T, DevError>;