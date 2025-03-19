/*
 * @Author: Peter/peterluck2021@163.com
 * @Date: 2025-03-12 22:05:47
 * @LastEditors: Peter/peterluck2021@163.com
 * @LastEditTime: 2025-03-19 14:34:10
 * @FilePath: /os/modules/net/src/portmap.rs
 * @Description: portmap module
 * 
 * Copyright (c) 2025 by peterluck2021@163.com, All Rights Reserved. 
 */

use alloc::collections::BTreeMap;
use smoltcp::wire::IpListenEndpoint;
use lazy_static::lazy_static; // 使用 lazy_static 宏
use spin::Mutex; // 假设使用 spin 库的 Mutex（确保线程安全）

type Port = u16;
type Fd = usize;

pub struct PortMap {
    map: BTreeMap<Port, (Fd, IpListenEndpoint)>,
}

// 使用 lazy_static 定义 pub static PORT_MAP
lazy_static! {
    pub static ref PORT_MAP: Mutex<PortMap> = Mutex::new(PortMap {
        map: BTreeMap::new(),
    });
}

impl PortMap {
    pub fn get(&self, port: Port) -> Option<(Fd, IpListenEndpoint)> {
        self.map.get(&port).cloned()
    }

    pub fn remove(&mut self, port: Port) {
        self.map.remove(&port);
    }

    pub fn insert(&mut self, port: Port, fd: Fd, listenport: IpListenEndpoint) {
        self.map.insert(port, (fd, listenport));
    }
}