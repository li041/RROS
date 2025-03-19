/*
 * @Author: Peter/peterluck2021@163.com
 * @Date: 2025-03-18 21:55:39
 * @LastEditors: Peter/peterluck2021@163.com
 * @LastEditTime: 2025-03-18 21:55:59
 * @FilePath: /net/src/SyncFlag.rs
 * @Description: Syncflag
 * 
 * Copyright (c) 2025 by peterluck2021@163.com, All Rights Reserved. 
 */
use core::sync::atomic::{AtomicBool, Ordering};
use core::hint::spin_loop;

pub struct SimpleSyncFlag {
    flag: AtomicBool,
}

impl SimpleSyncFlag {
    pub const fn new() -> Self {
        Self {
            flag: AtomicBool::new(false),
        }
    }

    /// 阻塞等待直到 flag 为 true（忙等待方式）
    pub fn wait(&self) {
        while !self.flag.load(Ordering::Acquire) {
            spin_loop();
        }
        // 等到通知后重置 flag（根据需要也可去掉重置逻辑）
        self.flag.store(false, Ordering::Release);
    }

    /// 通知等待者，将 flag 置为 true
    pub fn notify(&self) {
        self.flag.store(true, Ordering::Release);
    }
}
