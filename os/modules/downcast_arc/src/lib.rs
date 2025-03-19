#![no_main]
#![no_std]
#![feature(trait_upcasting)]
/*
 * @Author: Peter/peterluck2021@163.com
 * @Date: 2025-03-19 16:50:00
 * @LastEditors: Peter/peterluck2021@163.com
 * @LastEditTime: 2025-03-19 17:05:42
 * @FilePath: /os/modules/downcast_arc/src/lib.rs
 * @Description: downcast_arc
 * 
 * Copyright (c) 2025 by peterluck2021@163.com, All Rights Reserved. 
 */
/// @brief 将Arc<dyn xxx>转换为Arc<具体类型>的trait
// #![no_std]
// #![no_main]

extern crate alloc;
use core::any::Any;
use alloc::sync::Arc;

use core::option::Option::{None,Some};
use core::marker::{Send,Sync};
use core::option::Option;



pub trait DowncastArc: Any + Send + Sync {
    /// 请在具体类型中实现这个函数，返回self
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any>;

    /// @brief 将Arc<dyn xxx>转换为Arc<具体类型>
    /// 
    /// 如果Arc<dyn xxx>是Arc<具体类型>，则返回Some(Arc<具体类型>)，否则返回None
    /// 
    /// @param self Arc<dyn xxx>
    fn downcast_arc<T: Any + Send + Sync>(self: Arc<Self>) -> Option<Arc<T>> {
        let x: Arc<dyn Any> = self.as_any_arc();
        if x.is::<T>() {
            // into_raw不会改变引用计数
            let p = Arc::into_raw(x);
            let new = unsafe { Arc::from_raw(p as *const T) };
            return Some(new);
        }
        return None;
    }
}