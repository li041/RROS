/*
 * @Author: Peter/peterluck2021@163.com
 * @Date: 2025-03-17 09:34:59
 * @LastEditors: Peter/peterluck2021@163.com
 * @LastEditTime: 2025-03-18 14:56:24
 * @FilePath: /os/src/net/mod.rs
 * @Description: net mod define the sockettype socketdomain socketopt and transform from and to u16/usize
 * 
 * Copyright (c) 2025 by peterluck2021@163.com, All Rights Reserved. 
 */
use log::{self, warn};
use net::err::SysError;

pub mod addr;
pub mod socket;
mod unix;
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
 ///socket family
 pub enum SaFamily {
    AF_Unix=1,
    //IPV4
    AF_Inet=2,
    //IPV6
    AF_Inet6=10,

 }

 impl TryFrom<u16> for  SaFamily {
    type Error=SysError;
 
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1=>Ok(SaFamily::AF_Unix),
            2=>Ok(SaFamily::AF_Inet),
            10=>Ok(SaFamily::AF_Inet6),
            _=>Err(SysError::EINVAL)
        }
    }
 }

 ///match safamily to u16
 impl From<SaFamily> for u16 {
    fn from(value: SaFamily) -> Self {
        match value {
            SaFamily::AF_Unix=>1,
            SaFamily::AF_Inet=>2,
            SaFamily::AF_Inet6=>10
        }
    }
 }



 ///socket type for tranmit
 #[derive(Debug,Clone, Copy,PartialEq, Eq)]
 pub enum SocketType {
     STREAM=1,
     DGRAM=2,
     RAW=3,
     RDM=4,
     SEQPACKET=5,
     DCCP=6,
     PACKET=10,
 }

 impl TryFrom<u16> for SocketType {
    type Error=SysError;
 
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1=>Ok(SocketType::STREAM),
            2=>Ok(SocketType::DGRAM),
            3=>Ok(SocketType::RAW),
            4=>Ok(SocketType::RDM),
            5=>Ok(SocketType::SEQPACKET),
            6=>Ok(SocketType::DCCP),
            10=>Ok(SocketType::PACKET),
            level=>{
                warn!("[SocketType] unsupported option: {level}");
                Err(SysError::EINVAL)
            }
        }
    }
 }
 impl From<SocketType> for u16 {
    fn from(value: SocketType) -> Self {
        match value {
            SocketType::STREAM=>1,
            SocketType::DGRAM=>2,
            SocketType::RAW=>3,
            SocketType::RDM=>4,
            SocketType::SEQPACKET=>5,
            SocketType::DCCP=>6,
            SocketType::PACKET=>10   
        }
    }
 }


 #[derive(Debug,Clone, Copy,PartialEq, Eq)]
 #[allow(non_camel_case_types)]
 pub enum SocketLevel {
    IPPROTO_IP=0,
    SOL_SOCKET=1,
    IPPROTO_IPV6=41,
    IPPROTO_TCP=6,
 }

 impl TryFrom<usize> for SocketLevel {
    type Error=SysError;
 
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0=>Ok(SocketLevel::IPPROTO_IP),
            1=>Ok(SocketLevel::SOL_SOCKET),
            2=>Ok(SocketLevel::IPPROTO_TCP),
            41=>Ok(SocketLevel::IPPROTO_IPV6),
            level=>{
                warn!("[Socketlevel] unsupported option: {level}");
                Err(SysError::EINVAL)
            }
        }
    }
 }

 impl From<SocketLevel> for u16 {
    fn from(value: SocketLevel) -> Self {
        match value {
            SocketLevel::IPPROTO_IP=>0,
            SocketLevel::IPPROTO_IPV6=>41,
            SocketLevel::IPPROTO_TCP=>2,
            SocketLevel::SOL_SOCKET=>1,
        }
    }
 }

 ///
 ///https://www.man7.org/linux/man-pages/man7/socket.7.html
 #[derive(Debug,Clone, Copy,PartialEq, Eq)]
 #[allow(non_camel_case_types)]
 pub enum SocketOpt {
    ///Enable socket debugging.  Allowed only for processes with
    // the CAP_NET_ADMIN capability or an effective user ID of 0.
    DEBUG = 1,
    ///Indicates that the rules used in validating addresses
    // supplied in a bind(2) call should allow reuse of local
    // addresses.  For AF_INET sockets this means that a socket
    // may bind, except when there is an active listening socket
    // bound to the address.  When the listening socket is bound
    // to INADDR_ANY with a specific port then it is not possible
    // to bind to this port for any local address.  Argument is an
    // integer boolean flag.
    REUSEADDR = 2,
    /// Gets the socket type as an integer (e.g., SOCK_STREAM).
    // This socket option is read-only.
    TYPE = 3,
    ///Get and clear the pending socket error.  This socket option
    // is read-only.  Expects an integer.
    ERROR = 4,
    ///Don't send via a gateway, send only to directly connected
    // hosts.  The same effect can be achieved by setting the
    // MSG_DONTROUTE flag on a socket send(2) operation.  Expects
    // an integer boolean flag.
    DONTROUTE = 5,
    ///  Set or get the broadcast flag.  When enabled, datagram
    //   sockets are allowed to send packets to a broadcast address.
    //   This option has no effect on stream-oriented sockets.
    BROADCAST = 6,
    // Sets or gets the maximum socket send buffer in bytes.  The
    // kernel doubles this value (to allow space for bookkeeping
    // overhead) when it is set using setsockopt(2), and this
    // doubled value is returned by getsockopt(2).  The default
    // value is set by the /proc/sys/net/core/wmem_default file
    // and the maximum allowed value is set by the
    // /proc/sys/net/core/wmem_max file.  The minimum (doubled)
    // value for this option is 2048.
    SNDBUF = 7,///everytime the socket set sendbuf will send maxmuim data
    // Sets or gets the maximum socket receive buffer in bytes.
    // The kernel doubles this value (to allow space for
    // bookkeeping overhead) when it is set using setsockopt(2),
    // and this doubled value is returned by getsockopt(2).  The
    // default value is set by the /proc/sys/net/core/rmem_default
    // file, and the maximum allowed value is set by the
    // /proc/sys/net/core/rmem_max file.  The minimum (doubled)
    // value for this option is 256.
    RCVBUF = 8,///everytime the socket set sendbuf will recv maxmuim data
    ///Enable sending of keep-alive messages on connection-
    // oriented sockets.  Expects an integer boolean flag.
    KEEPALIVE = 9,
    ///If this option is enabled, out-of-band data is directly
    // placed into the receive data stream.  Otherwise, out-of-
    // band data is passed only when the MSG_OOB flag is set
    // during receiving.
    OOBINLINE = 10,
    ///
    NO_CHECK = 11,
    ///Priority to be used should first use for protocol defined first
    ///Set the protocol-defined priority for all packets to be
    // sent on this socket.  Linux uses this value to order the
    // networking queues: packets with a higher priority may be
    // processed first depending on the selected device queueing
    // discipline.  Setting a priority outside the range 0 to 6
    // requires the CAP_NET_ADMIN capability.
    PRIORITY = 12,
    ///Sets or gets the SO_LINGER option.  The argument is a
    //     linger structure.

    //     struct linger {
    //         int l_onoff;    /* linger active */
    //         int l_linger;   /* how many seconds to linger for */
    //     };
    // When enabled, a close(2) or shutdown(2) will not return
    // until all queued messages for the socket have been
    // successfully sent or the linger timeout has been reached.
    // Otherwise, the call returns immediately and the closing is
    // done in the background.  When the socket is closed as part
    // of exit(2), it always lingers in the background.
    // this option will make sure that whether to wait all queue socket to be done
    LINGER = 13,
    ///Enable BSD bug-to-bug compatibility.  This is used by the
    // UDP protocol module in Linux 2.0 and 2.2.  If enabled, ICMP
    // errors received for a UDP socket will not be passed to the
    // user program.  In later kernel versions, support for this
    // option has been phased out: Linux 2.4 silently ignores it,
    // and Linux 2.6 generates a kernel warning (printk()) if a
    // program uses this option.  Linux 2.0 also enabled BSD bug-
    // to-bug compatibility options (random header changing,
    // skipping of the broadcast flag) for raw sockets with this
    // option, but that was removed in Linux 2.2.
    BSDCOMPAT = 14,
    ///Permits multiple AF_INET or AF_INET6 sockets to be bound to
    // an identical socket address.  This option must be set on
    // each socket (including the first socket) prior to calling
    // bind(2) on the socket.  To prevent port hijacking, all of
    // the processes binding to the same address must have the
    // same effective UID.  This option can be employed with both
    // TCP and UDP sockets.
    // For TCP sockets, this option allows accept(2) load
    // distribution in a multi-threaded server to be improved by
    // using a distinct listener socket for each thread.  This
    // provides improved load distribution as compared to
    // traditional techniques such using a single accept(2)ing
    // thread that distributes connections, or having multiple
    // threads that compete to accept(2) from the same socket.
    // For UDP sockets, the use of this option can provide better
    // distribution of incoming datagrams to multiple processes
    // (or threads) as compared to the traditional technique of
    // having multiple processes compete to receive datagrams on
    // the same socket.
    REUSEPORT = 15,
    PASSCRED = 16,
    PEERCRED = 17,
    RCVLOWAT = 18,
    SNDLOWAT = 19,
    RCVTIMEO_OLD = 20,
    SNDTIMEO_OLD = 21,
    SECURITY_AUTHENTICATION = 22,
    SECURITY_ENCRYPTION_TRANSPORT = 23,
    SECURITY_ENCRYPTION_NETWORK = 24,
    /// Bind this socket to a particular device like “eth0”, as specified in the
    /// passed interface name
    BINDTODEVICE = 25,
    ATTACH_FILTER = 26,
    DETACH_FILTER = 27,
    SNDBUFFORCE = 32,
    RCVBUFFORCE = 33,
     
 }

 impl TryFrom<usize> for SocketOpt {
    type Error=SysError;
 
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value{
            1=>Ok(SocketOpt::DEBUG),
            2=>Ok(SocketOpt::REUSEADDR),
            3=>Ok(SocketOpt::TYPE),
            4=>Ok(SocketOpt::ERROR),
            5=>Ok(SocketOpt::DONTROUTE),
            6=>Ok(SocketOpt::BROADCAST),
            7=>Ok(SocketOpt::SNDBUF),
            8=>Ok(SocketOpt::RCVBUF),
            9=>Ok(SocketOpt::KEEPALIVE),
            10=>Ok(SocketOpt::OOBINLINE),
            11=>Ok(SocketOpt::NO_CHECK),
            12=>Ok(SocketOpt::PRIORITY),
            13=>Ok(SocketOpt::LINGER),
            14=>Ok(SocketOpt::BSDCOMPAT),
            15=>Ok(SocketOpt::REUSEPORT),
            16=>Ok(SocketOpt::PASSCRED),
            17=>Ok(SocketOpt::PEERCRED),
            18=>Ok(SocketOpt::RCVLOWAT),
            19=>Ok(SocketOpt::SNDLOWAT),
            20=>Ok(SocketOpt::RCVTIMEO_OLD),
            21=>Ok(SocketOpt::SNDTIMEO_OLD),
            22=>Ok(SocketOpt::SECURITY_AUTHENTICATION),
            23=>Ok(SocketOpt::SECURITY_ENCRYPTION_TRANSPORT),
            24=>Ok(SocketOpt::SECURITY_ENCRYPTION_NETWORK),
            25=>Ok(SocketOpt::BINDTODEVICE),
            26=>Ok(SocketOpt::ATTACH_FILTER),
            27=>Ok(SocketOpt::DETACH_FILTER),
            32=>Ok(SocketOpt::SNDBUFFORCE),
            33=>Ok(SocketOpt::RCVBUFFORCE),
            level=>{
                warn!("[SocketOpt] unsupported option: {level}");
                Ok(SocketOpt::DEBUG)
            }
            
        }
    }
 }

 #[derive(Debug,PartialEq, Eq,Clone, Copy)]
 pub enum TcpSocketOpt {
     NODELAY=1,
     MAXSEG=2,
     INFO=11,
     CONGESTION=13,
 }

 impl TryFrom<usize> for TcpSocketOpt {
    type Error=SysError;
 
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            1=>Ok(TcpSocketOpt::NODELAY),
            2=>Ok(TcpSocketOpt::MAXSEG),
            11=>Ok(TcpSocketOpt::INFO),
            13=>Ok(TcpSocketOpt::CONGESTION),
            level=>{
                warn!("[TcpSocketOpt] unsupported option: {level}");
                Err(SysError::EINVAL)
            }
        }
    }
 }










 