/*
 * @Author: peter
 * @Date: 2025-03-06 20:22:04
 * @LastEditors: peter
 * @LastEditTime: 2025-03-06 20:50:23
 * @FilePath: /RROS/user/src/lang_items.rs
 * @Description: 
 * 
 * Copyright (c) 2025 by ${git_name_email}, All Rights Reserved. 
 */
use super::exit;

#[panic_handler]
fn panic_handler(panic_info: &core::panic::PanicInfo) -> ! {
    let err = panic_info.message();
    if let Some(location) = panic_info.location() {
        println!(
            "Panicked at {}:{}, {}",
            location.file(),
            location.line(),
            err
        );
    } else {
        println!("Panicked: {}", err);
    }
    exit(-1);
}
