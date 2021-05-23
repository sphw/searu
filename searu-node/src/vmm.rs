// Adapted from  https://github.com/cloud-hypervisor/cloud-hypervisor/blob/master/net_util/src/mac.rs and
// https://github.com/cloud-hypervisor/cloud-hypervisor/blob/master/vmm/src/api/mod.rs
//
// Copyright © 2019 Intel Corporation
//
// SPDX-License-Identifier: Apache-2.0
//
//
//

use serde::de::{Deserializer, Error as SerdeError};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::convert::From;
use std::fmt;
use std::io;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::result;
use std::str::FromStr;

pub const DEFAULT_VCPUS: u8 = 1;
pub const DEFAULT_MEMORY_MB: u64 = 512;
pub const DEFAULT_RNG_SOURCE: &str = "/dev/urandom";
pub const DEFAULT_NUM_QUEUES_VUNET: usize = 2;
pub const DEFAULT_QUEUE_SIZE_VUNET: u16 = 256;
pub const DEFAULT_NUM_QUEUES_VUBLK: usize = 1;
pub const DEFAULT_QUEUE_SIZE_VUBLK: u16 = 128;

/// Errors associated with VM configuration parameters.
#[derive(Debug)]
pub enum Error {
    #[cfg(feature = "tdx")]
    /// Failed to parse TDX config
    ParseTdx(OptionParserError),
    #[cfg(feature = "tdx")]
    // No TDX firmware
    FirmwarePathMissing,
}

pub type Result<T> = result::Result<T, Error>;

pub struct VmParams<'a> {
    pub cpus: &'a str,
    pub memory: &'a str,
    pub memory_zones: Option<Vec<&'a str>>,
    pub kernel: Option<&'a str>,
    pub initramfs: Option<&'a str>,
    pub cmdline: Option<&'a str>,
    pub disks: Option<Vec<&'a str>>,
    pub net: Option<Vec<&'a str>>,
    pub rng: &'a str,
    pub balloon: Option<&'a str>,
    pub fs: Option<Vec<&'a str>>,
    pub pmem: Option<Vec<&'a str>>,
    pub serial: &'a str,
    pub console: &'a str,
    pub devices: Option<Vec<&'a str>>,
    pub vsock: Option<&'a str>,
    #[cfg(target_arch = "x86_64")]
    pub sgx_epc: Option<Vec<&'a str>>,
    pub numa: Option<Vec<&'a str>>,
    pub watchdog: bool,
    #[cfg(feature = "tdx")]
    pub tdx: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum HotplugMethod {
    Acpi,
    VirtioMem,
}

impl Default for HotplugMethod {
    fn default() -> Self {
        HotplugMethod::Acpi
    }
}

#[derive(Debug)]
pub enum ParseHotplugMethodError {
    InvalidValue(String),
}

impl FromStr for HotplugMethod {
    type Err = ParseHotplugMethodError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "acpi" => Ok(HotplugMethod::Acpi),
            "virtio-mem" => Ok(HotplugMethod::VirtioMem),
            _ => Err(ParseHotplugMethodError::InvalidValue(s.to_owned())),
        }
    }
}

pub enum CpuTopologyParseError {
    InvalidValue(String),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct CpuTopology {
    pub threads_per_core: u8,
    pub cores_per_die: u8,
    pub dies_per_package: u8,
    pub packages: u8,
}

impl FromStr for CpuTopology {
    type Err = CpuTopologyParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();

        if parts.len() != 4 {
            return Err(Self::Err::InvalidValue(s.to_owned()));
        }

        let t = CpuTopology {
            threads_per_core: parts[0]
                .parse()
                .map_err(|_| Self::Err::InvalidValue(s.to_owned()))?,
            cores_per_die: parts[1]
                .parse()
                .map_err(|_| Self::Err::InvalidValue(s.to_owned()))?,
            dies_per_package: parts[2]
                .parse()
                .map_err(|_| Self::Err::InvalidValue(s.to_owned()))?,
            packages: parts[3]
                .parse()
                .map_err(|_| Self::Err::InvalidValue(s.to_owned()))?,
        };

        Ok(t)
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct CpusConfig {
    pub boot_vcpus: u8,
    pub max_vcpus: u8,
    #[serde(default)]
    pub topology: Option<CpuTopology>,
    #[serde(default)]
    pub kvm_hyperv: bool,
    #[serde(default)]
    pub max_phys_bits: Option<u8>,
}

impl Default for CpusConfig {
    fn default() -> Self {
        CpusConfig {
            boot_vcpus: DEFAULT_VCPUS,
            max_vcpus: DEFAULT_VCPUS,
            topology: None,
            kvm_hyperv: false,
            max_phys_bits: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct MemoryZoneConfig {
    pub id: String,
    pub size: u64,
    #[serde(default)]
    pub file: Option<PathBuf>,
    #[serde(default)]
    pub shared: bool,
    #[serde(default)]
    pub hugepages: bool,
    #[serde(default)]
    pub hugepage_size: Option<u64>,
    #[serde(default)]
    pub host_numa_node: Option<u32>,
    #[serde(default)]
    pub hotplug_size: Option<u64>,
    #[serde(default)]
    pub hotplugged_size: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct MemoryConfig {
    pub size: u64,
    #[serde(default)]
    pub mergeable: bool,
    #[serde(default)]
    pub hotplug_method: HotplugMethod,
    #[serde(default)]
    pub hotplug_size: Option<u64>,
    #[serde(default)]
    pub hotplugged_size: Option<u64>,
    #[serde(default)]
    pub shared: bool,
    #[serde(default)]
    pub hugepages: bool,
    #[serde(default)]
    pub hugepage_size: Option<u64>,
    #[serde(default)]
    pub zones: Option<Vec<MemoryZoneConfig>>,
}

impl MemoryConfig {
    pub fn total_size(&self) -> u64 {
        let mut size = self.size;
        if let Some(hotplugged_size) = self.hotplugged_size {
            size += hotplugged_size;
        }

        if let Some(zones) = &self.zones {
            for zone in zones.iter() {
                size += zone.size;
                if let Some(hotplugged_size) = zone.hotplugged_size {
                    size += hotplugged_size;
                }
            }
        }

        size
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        MemoryConfig {
            size: DEFAULT_MEMORY_MB << 20,
            mergeable: false,
            hotplug_method: HotplugMethod::Acpi,
            hotplug_size: None,
            hotplugged_size: None,
            shared: false,
            hugepages: false,
            hugepage_size: None,
            zones: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct KernelConfig {
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct InitramfsConfig {
    pub path: PathBuf,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct CmdlineConfig {
    pub args: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct DiskConfig {
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub readonly: bool,
    #[serde(default)]
    pub direct: bool,
    #[serde(default)]
    pub iommu: bool,
    #[serde(default = "default_diskconfig_num_queues")]
    pub num_queues: usize,
    #[serde(default = "default_diskconfig_queue_size")]
    pub queue_size: u16,
    #[serde(default)]
    pub vhost_user: bool,
    pub vhost_socket: Option<String>,
    #[serde(default = "default_diskconfig_poll_queue")]
    pub poll_queue: bool,
    #[serde(default)]
    pub rate_limiter_config: Option<RateLimiterConfig>,
    #[serde(default)]
    pub id: Option<String>,
    // For testing use only. Not exposed in API.
    #[serde(default)]
    pub disable_io_uring: bool,
}

fn default_diskconfig_num_queues() -> usize {
    DEFAULT_NUM_QUEUES_VUBLK
}

fn default_diskconfig_queue_size() -> u16 {
    DEFAULT_QUEUE_SIZE_VUBLK
}

fn default_diskconfig_poll_queue() -> bool {
    true
}

impl Default for DiskConfig {
    fn default() -> Self {
        Self {
            path: None,
            readonly: false,
            direct: false,
            iommu: false,
            num_queues: default_diskconfig_num_queues(),
            queue_size: default_diskconfig_queue_size(),
            vhost_user: false,
            vhost_socket: None,
            poll_queue: default_diskconfig_poll_queue(),
            id: None,
            disable_io_uring: false,
            rate_limiter_config: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum VhostMode {
    Client,
    Server,
}

impl Default for VhostMode {
    fn default() -> Self {
        VhostMode::Client
    }
}

#[derive(Debug)]
pub enum ParseVhostModeError {
    InvalidValue(String),
}

impl FromStr for VhostMode {
    type Err = ParseVhostModeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "client" => Ok(VhostMode::Client),
            "server" => Ok(VhostMode::Server),
            _ => Err(ParseVhostModeError::InvalidValue(s.to_owned())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NetConfig {
    #[serde(default = "default_netconfig_tap")]
    pub tap: Option<String>,
    #[serde(default = "default_netconfig_ip")]
    pub ip: Ipv4Addr,
    #[serde(default = "default_netconfig_mask")]
    pub mask: Ipv4Addr,
    #[serde(default = "default_netconfig_mac")]
    pub mac: MacAddr,
    #[serde(default)]
    pub host_mac: Option<MacAddr>,
    #[serde(default)]
    pub iommu: bool,
    #[serde(default = "default_netconfig_num_queues")]
    pub num_queues: usize,
    #[serde(default = "default_netconfig_queue_size")]
    pub queue_size: u16,
    #[serde(default)]
    pub vhost_user: bool,
    pub vhost_socket: Option<String>,
    #[serde(default)]
    pub vhost_mode: VhostMode,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub fds: Option<Vec<i32>>,
    #[serde(default)]
    pub rate_limiter_config: Option<RateLimiterConfig>,
}

fn default_netconfig_tap() -> Option<String> {
    None
}

fn default_netconfig_ip() -> Ipv4Addr {
    Ipv4Addr::new(192, 168, 249, 1)
}

fn default_netconfig_mask() -> Ipv4Addr {
    Ipv4Addr::new(255, 255, 255, 0)
}

fn default_netconfig_mac() -> MacAddr {
    MacAddr::local_random()
}

fn default_netconfig_num_queues() -> usize {
    DEFAULT_NUM_QUEUES_VUNET
}

fn default_netconfig_queue_size() -> u16 {
    DEFAULT_QUEUE_SIZE_VUNET
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            tap: default_netconfig_tap(),
            ip: default_netconfig_ip(),
            mask: default_netconfig_mask(),
            mac: default_netconfig_mac(),
            host_mac: None,
            iommu: false,
            num_queues: default_netconfig_num_queues(),
            queue_size: default_netconfig_queue_size(),
            vhost_user: false,
            vhost_socket: None,
            vhost_mode: VhostMode::Client,
            id: None,
            fds: None,
            rate_limiter_config: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct RngConfig {
    pub src: PathBuf,
    #[serde(default)]
    pub iommu: bool,
}

impl Default for RngConfig {
    fn default() -> Self {
        RngConfig {
            src: PathBuf::from(DEFAULT_RNG_SOURCE),
            iommu: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct BalloonConfig {
    pub size: u64,
}

impl BalloonConfig {
    pub const SYNTAX: &'static str = "Balloon parameters \"size=<balloon_size>\"";
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct FsConfig {
    pub tag: String,
    pub socket: PathBuf,
    #[serde(default = "default_fsconfig_num_queues")]
    pub num_queues: usize,
    #[serde(default = "default_fsconfig_queue_size")]
    pub queue_size: u16,
    #[serde(default = "default_fsconfig_dax")]
    pub dax: bool,
    #[serde(default = "default_fsconfig_cache_size")]
    pub cache_size: u64,
    #[serde(default)]
    pub id: Option<String>,
}

fn default_fsconfig_num_queues() -> usize {
    1
}

fn default_fsconfig_queue_size() -> u16 {
    1024
}

fn default_fsconfig_dax() -> bool {
    true
}

fn default_fsconfig_cache_size() -> u64 {
    0x0002_0000_0000
}

impl Default for FsConfig {
    fn default() -> Self {
        Self {
            tag: "".to_owned(),
            socket: PathBuf::new(),
            num_queues: default_fsconfig_num_queues(),
            queue_size: default_fsconfig_queue_size(),
            dax: default_fsconfig_dax(),
            cache_size: default_fsconfig_cache_size(),
            id: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
pub struct PmemConfig {
    pub file: PathBuf,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub iommu: bool,
    #[serde(default)]
    pub mergeable: bool,
    #[serde(default)]
    pub discard_writes: bool,
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum ConsoleOutputMode {
    Off,
    Pty,
    Tty,
    File,
    Null,
}

impl ConsoleOutputMode {
    pub fn input_enabled(&self) -> bool {
        matches!(self, ConsoleOutputMode::Tty | ConsoleOutputMode::Pty)
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ConsoleConfig {
    #[serde(default = "default_consoleconfig_file")]
    pub file: Option<PathBuf>,
    pub mode: ConsoleOutputMode,
    #[serde(default)]
    pub iommu: bool,
}

fn default_consoleconfig_file() -> Option<PathBuf> {
    None
}

impl ConsoleConfig {
    pub fn default_serial() -> Self {
        ConsoleConfig {
            file: None,
            mode: ConsoleOutputMode::Null,
            iommu: false,
        }
    }

    pub fn default_console() -> Self {
        ConsoleConfig {
            file: None,
            mode: ConsoleOutputMode::Tty,
            iommu: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
pub struct DeviceConfig {
    pub path: PathBuf,
    #[serde(default)]
    pub iommu: bool,
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
pub struct VsockConfig {
    pub cid: u64,
    pub socket: PathBuf,
    #[serde(default)]
    pub iommu: bool,
    #[serde(default)]
    pub id: Option<String>,
}

#[cfg(feature = "tdx")]
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
pub struct TdxConfig {
    pub firmware: PathBuf,
}

#[cfg(feature = "tdx")]
impl TdxConfig {
    pub fn parse(tdx: &str) -> Result<Self> {
        let mut parser = OptionParser::new();
        parser.add("firmware");
        parser.parse(tdx).map_err(Error::ParseTdx)?;
        let firmware = parser
            .get("firmware")
            .map(PathBuf::from)
            .ok_or(Error::FirmwarePathMissing)?;
        Ok(TdxConfig { firmware })
    }
}

#[cfg(target_arch = "x86_64")]
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
pub struct SgxEpcConfig {
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub prefault: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
pub struct NumaDistance {
    #[serde(default)]
    pub destination: u32,
    #[serde(default)]
    pub distance: u8,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
pub struct NumaConfig {
    #[serde(default)]
    pub guest_numa_id: u32,
    #[serde(default)]
    pub cpus: Option<Vec<u8>>,
    #[serde(default)]
    pub distances: Option<Vec<NumaDistance>>,
    #[serde(default)]
    pub memory_zones: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
pub struct RestoreConfig {
    pub source_url: PathBuf,
    #[serde(default)]
    pub prefault: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct VmConfig {
    #[serde(default)]
    pub cpus: CpusConfig,
    #[serde(default)]
    pub memory: MemoryConfig,
    pub kernel: Option<KernelConfig>,
    #[serde(default)]
    pub initramfs: Option<InitramfsConfig>,
    #[serde(default)]
    pub cmdline: CmdlineConfig,
    pub disks: Option<Vec<DiskConfig>>,
    pub net: Option<Vec<NetConfig>>,
    #[serde(default)]
    pub rng: RngConfig,
    pub balloon: Option<BalloonConfig>,
    pub fs: Option<Vec<FsConfig>>,
    pub pmem: Option<Vec<PmemConfig>>,
    #[serde(default = "ConsoleConfig::default_serial")]
    pub serial: ConsoleConfig,
    #[serde(default = "ConsoleConfig::default_console")]
    pub console: ConsoleConfig,
    pub devices: Option<Vec<DeviceConfig>>,
    pub vsock: Option<VsockConfig>,
    #[serde(default)]
    pub iommu: bool,
    #[cfg(target_arch = "x86_64")]
    pub sgx_epc: Option<Vec<SgxEpcConfig>>,
    pub numa: Option<Vec<NumaConfig>>,
    #[serde(default)]
    pub watchdog: bool,
    #[cfg(feature = "tdx")]
    pub tdx: Option<TdxConfig>,
}

pub const MAC_ADDR_LEN: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MacAddr {
    bytes: [u8; MAC_ADDR_LEN],
}

impl MacAddr {
    pub fn parse_str<S>(s: &S) -> result::Result<MacAddr, io::Error>
    where
        S: AsRef<str> + ?Sized,
    {
        let v: Vec<&str> = s.as_ref().split(':').collect();
        let mut bytes = [0u8; MAC_ADDR_LEN];
        let common_err = Err(io::Error::new(
            io::ErrorKind::Other,
            format!("parsing of {} into a MAC address failed", s.as_ref()),
        ));

        if v.len() != MAC_ADDR_LEN {
            return common_err;
        }

        for i in 0..MAC_ADDR_LEN {
            if v[i].len() != 2 {
                return common_err;
            }
            bytes[i] = u8::from_str_radix(v[i], 16).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("parsing of {} into a MAC address failed: {}", s.as_ref(), e),
                )
            })?;
        }

        Ok(MacAddr { bytes })
    }

    // Does not check whether src.len() == MAC_ADDR_LEN.
    #[inline]
    pub fn from_bytes_unchecked(src: &[u8]) -> MacAddr {
        // TODO: using something like std::mem::uninitialized could avoid the extra initialization,
        // if this ever becomes a performance bottleneck.
        let mut bytes = [0u8; MAC_ADDR_LEN];
        bytes[..].copy_from_slice(&src);

        MacAddr { bytes }
    }

    // An error can only occur if the slice length is different from MAC_ADDR_LEN.
    #[inline]
    pub fn from_bytes(src: &[u8]) -> result::Result<MacAddr, io::Error> {
        if src.len() != MAC_ADDR_LEN {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("invalid length of slice: {} vs {}", src.len(), MAC_ADDR_LEN),
            ));
        }
        Ok(MacAddr::from_bytes_unchecked(src))
    }

    #[inline]
    pub fn get_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn local_random() -> MacAddr {
        todo!()
    }

    // pub fn local_random() -> MacAddr {
    //     // Generate a fully random MAC
    //     let mut random_bytes = [0u8; MAC_ADDR_LEN];
    //     unsafe {
    //         // Man page says this function will not be interrupted by a signal
    //         // for requests less than 256 bytes
    //         if libc::getrandom(
    //             random_bytes.as_mut_ptr() as *mut _ as *mut libc::c_void,
    //             MAC_ADDR_LEN,
    //             0,
    //         ) < 0
    //         {
    //             panic!(
    //                 "Error populating MAC address with random data: {}",
    //                 std::io::Error::last_os_error()
    //             )
    //         }
    //     };

    //     // Set the first byte to make the OUI a locally administered OUI
    //     random_bytes[0] = 0x2e;

    //     MacAddr {
    //         bytes: random_bytes,
    //     }
    // }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let b = &self.bytes;
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            b[0], b[1], b[2], b[3], b[4], b[5]
        )
    }
}

impl Serialize for MacAddr {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MacAddr {
    fn deserialize<D>(deserializer: D) -> result::Result<MacAddr, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        MacAddr::parse_str(&s)
            .map_err(|e| D::Error::custom(format!("The provided MAC address is invalid: {}", e)))
    }
}

pub enum MacAddrParseError {
    InvalidValue(String),
}

impl FromStr for MacAddr {
    type Err = MacAddrParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        MacAddr::parse_str(s).map_err(|_| MacAddrParseError::InvalidValue(s.to_owned()))
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct TokenBucketConfig {
    pub size: u64,
    pub one_time_burst: Option<u64>,
    pub refill_time: u64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RateLimiterConfig {
    pub bandwidth: Option<TokenBucketConfig>,
    pub ops: Option<TokenBucketConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_parser() {
        let mut parser = OptionParser::new();
        parser
            .add("size")
            .add("mergeable")
            .add("hotplug_method")
            .add("hotplug_size");

        assert!(parser.parse("size=128M,hanging_param").is_err());
        assert!(parser.parse("size=128M,too_many_equals=foo=bar").is_err());
        assert!(parser.parse("size=128M,file=/dev/shm").is_err());
        assert!(parser.parse("size=128M").is_ok());

        assert_eq!(parser.get("size"), Some("128M".to_owned()));
        assert!(!parser.is_set("mergeable"));
        assert!(parser.is_set("size"));
    }

    #[test]
    fn test_cpu_parsing() -> Result<()> {
        assert_eq!(CpusConfig::parse("")?, CpusConfig::default());

        assert_eq!(
            CpusConfig::parse("boot=1")?,
            CpusConfig {
                boot_vcpus: 1,
                max_vcpus: 1,
                ..Default::default()
            }
        );
        assert_eq!(
            CpusConfig::parse("boot=1,max=2")?,
            CpusConfig {
                boot_vcpus: 1,
                max_vcpus: 2,
                ..Default::default()
            }
        );
        assert_eq!(
            CpusConfig::parse("boot=8,topology=2:2:1:2")?,
            CpusConfig {
                boot_vcpus: 8,
                max_vcpus: 8,
                topology: Some(CpuTopology {
                    threads_per_core: 2,
                    cores_per_die: 2,
                    dies_per_package: 1,
                    packages: 2
                }),
                ..Default::default()
            }
        );

        assert!(CpusConfig::parse("boot=8,topology=2:2:1").is_err());
        assert!(CpusConfig::parse("boot=8,topology=2:2:1:x").is_err());
        assert_eq!(
            CpusConfig::parse("boot=1,kvm_hyperv=on")?,
            CpusConfig {
                boot_vcpus: 1,
                max_vcpus: 1,
                kvm_hyperv: true,
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn test_mem_parsing() -> Result<()> {
        assert_eq!(MemoryConfig::parse("", None)?, MemoryConfig::default());
        // Default string
        assert_eq!(
            MemoryConfig::parse("size=512M", None)?,
            MemoryConfig::default()
        );
        assert_eq!(
            MemoryConfig::parse("size=512M,mergeable=on", None)?,
            MemoryConfig {
                size: 512 << 20,
                mergeable: true,
                ..Default::default()
            }
        );
        assert_eq!(
            MemoryConfig::parse("mergeable=on", None)?,
            MemoryConfig {
                mergeable: true,
                ..Default::default()
            }
        );
        assert_eq!(
            MemoryConfig::parse("size=1G,mergeable=off", None)?,
            MemoryConfig {
                size: 1 << 30,
                mergeable: false,
                ..Default::default()
            }
        );
        assert_eq!(
            MemoryConfig::parse("hotplug_method=acpi", None)?,
            MemoryConfig {
                ..Default::default()
            }
        );
        assert_eq!(
            MemoryConfig::parse("hotplug_method=acpi,hotplug_size=512M", None)?,
            MemoryConfig {
                hotplug_size: Some(512 << 20),
                ..Default::default()
            }
        );
        assert_eq!(
            MemoryConfig::parse("hotplug_method=virtio-mem,hotplug_size=512M", None)?,
            MemoryConfig {
                hotplug_size: Some(512 << 20),
                hotplug_method: HotplugMethod::VirtioMem,
                ..Default::default()
            }
        );
        assert_eq!(
            MemoryConfig::parse("hugepages=on,size=1G,hugepage_size=2M", None)?,
            MemoryConfig {
                hugepage_size: Some(2 << 20),
                size: 1 << 30,
                hugepages: true,
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn test_disk_parsing() -> Result<()> {
        assert_eq!(
            DiskConfig::parse("path=/path/to_file")?,
            DiskConfig {
                path: Some(PathBuf::from("/path/to_file")),
                ..Default::default()
            }
        );
        assert_eq!(
            DiskConfig::parse("path=/path/to_file,id=mydisk0")?,
            DiskConfig {
                path: Some(PathBuf::from("/path/to_file")),
                id: Some("mydisk0".to_owned()),
                ..Default::default()
            }
        );
        assert_eq!(
            DiskConfig::parse("vhost_user=true,socket=/tmp/sock")?,
            DiskConfig {
                vhost_socket: Some(String::from("/tmp/sock")),
                vhost_user: true,
                ..Default::default()
            }
        );
        assert_eq!(
            DiskConfig::parse("path=/path/to_file,iommu=on")?,
            DiskConfig {
                path: Some(PathBuf::from("/path/to_file")),
                iommu: true,
                ..Default::default()
            }
        );
        assert_eq!(
            DiskConfig::parse("path=/path/to_file,iommu=on,queue_size=256")?,
            DiskConfig {
                path: Some(PathBuf::from("/path/to_file")),
                iommu: true,
                queue_size: 256,
                ..Default::default()
            }
        );
        assert_eq!(
            DiskConfig::parse("path=/path/to_file,iommu=on,queue_size=256,num_queues=4")?,
            DiskConfig {
                path: Some(PathBuf::from("/path/to_file")),
                iommu: true,
                queue_size: 256,
                num_queues: 4,
                ..Default::default()
            }
        );
        assert_eq!(
            DiskConfig::parse("path=/path/to_file,direct=on")?,
            DiskConfig {
                path: Some(PathBuf::from("/path/to_file")),
                direct: true,
                ..Default::default()
            }
        );
        assert_eq!(
            DiskConfig::parse("path=/path/to_file")?,
            DiskConfig {
                path: Some(PathBuf::from("/path/to_file")),
                ..Default::default()
            }
        );
        assert_eq!(
            DiskConfig::parse("path=/path/to_file")?,
            DiskConfig {
                path: Some(PathBuf::from("/path/to_file")),
                ..Default::default()
            }
        );
        assert_eq!(
            DiskConfig::parse("path=/path/to_file,poll_queue=false")?,
            DiskConfig {
                path: Some(PathBuf::from("/path/to_file")),
                poll_queue: false,
                ..Default::default()
            }
        );
        assert_eq!(
            DiskConfig::parse("path=/path/to_file,poll_queue=true")?,
            DiskConfig {
                path: Some(PathBuf::from("/path/to_file")),
                poll_queue: true,
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn test_net_parsing() -> Result<()> {
        // mac address is random
        assert_eq!(
            NetConfig::parse("mac=de:ad:be:ef:12:34,host_mac=12:34:de:ad:be:ef")?,
            NetConfig {
                mac: MacAddr::parse_str("de:ad:be:ef:12:34").unwrap(),
                host_mac: Some(MacAddr::parse_str("12:34:de:ad:be:ef").unwrap()),
                ..Default::default()
            }
        );

        assert_eq!(
            NetConfig::parse("mac=de:ad:be:ef:12:34,host_mac=12:34:de:ad:be:ef,id=mynet0")?,
            NetConfig {
                mac: MacAddr::parse_str("de:ad:be:ef:12:34").unwrap(),
                host_mac: Some(MacAddr::parse_str("12:34:de:ad:be:ef").unwrap()),
                id: Some("mynet0".to_owned()),
                ..Default::default()
            }
        );

        assert_eq!(
            NetConfig::parse(
                "mac=de:ad:be:ef:12:34,host_mac=12:34:de:ad:be:ef,tap=tap0,ip=192.168.100.1,mask=255.255.255.128"
            )?,
            NetConfig {
                mac: MacAddr::parse_str("de:ad:be:ef:12:34").unwrap(),
                host_mac: Some(MacAddr::parse_str("12:34:de:ad:be:ef").unwrap()),
                tap: Some("tap0".to_owned()),
                ip: "192.168.100.1".parse().unwrap(),
                mask: "255.255.255.128".parse().unwrap(),
                ..Default::default()
            }
        );

        assert_eq!(
            NetConfig::parse(
                "mac=de:ad:be:ef:12:34,host_mac=12:34:de:ad:be:ef,vhost_user=true,socket=/tmp/sock"
            )?,
            NetConfig {
                mac: MacAddr::parse_str("de:ad:be:ef:12:34").unwrap(),
                host_mac: Some(MacAddr::parse_str("12:34:de:ad:be:ef").unwrap()),
                vhost_user: true,
                vhost_socket: Some("/tmp/sock".to_owned()),
                ..Default::default()
            }
        );

        assert_eq!(
            NetConfig::parse("mac=de:ad:be:ef:12:34,host_mac=12:34:de:ad:be:ef,num_queues=4,queue_size=1024,iommu=on")?,
            NetConfig {
                mac: MacAddr::parse_str("de:ad:be:ef:12:34").unwrap(),
                host_mac: Some(MacAddr::parse_str("12:34:de:ad:be:ef").unwrap()),
                num_queues: 4,
                queue_size: 1024,
                iommu: true,
                ..Default::default()
            }
        );

        assert_eq!(
            NetConfig::parse("mac=de:ad:be:ef:12:34,fd=3:7,num_queues=4")?,
            NetConfig {
                mac: MacAddr::parse_str("de:ad:be:ef:12:34").unwrap(),
                fds: Some(vec![3, 7]),
                num_queues: 4,
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_rng() -> Result<()> {
        assert_eq!(RngConfig::parse("")?, RngConfig::default());
        assert_eq!(
            RngConfig::parse("src=/dev/random")?,
            RngConfig {
                src: PathBuf::from("/dev/random"),
                ..Default::default()
            }
        );
        assert_eq!(
            RngConfig::parse("src=/dev/random,iommu=on")?,
            RngConfig {
                src: PathBuf::from("/dev/random"),
                iommu: true,
            }
        );
        assert_eq!(
            RngConfig::parse("iommu=on")?,
            RngConfig {
                iommu: true,
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn test_parse_fs() -> Result<()> {
        // "tag" and "socket" must be supplied
        assert!(FsConfig::parse("").is_err());
        assert!(FsConfig::parse("tag=mytag").is_err());
        assert!(FsConfig::parse("socket=/tmp/sock").is_err());
        assert_eq!(
            FsConfig::parse("tag=mytag,socket=/tmp/sock")?,
            FsConfig {
                socket: PathBuf::from("/tmp/sock"),
                tag: "mytag".to_owned(),
                ..Default::default()
            }
        );
        assert_eq!(
            FsConfig::parse("tag=mytag,socket=/tmp/sock")?,
            FsConfig {
                socket: PathBuf::from("/tmp/sock"),
                tag: "mytag".to_owned(),
                ..Default::default()
            }
        );
        assert_eq!(
            FsConfig::parse("tag=mytag,socket=/tmp/sock,num_queues=4,queue_size=1024")?,
            FsConfig {
                socket: PathBuf::from("/tmp/sock"),
                tag: "mytag".to_owned(),
                num_queues: 4,
                queue_size: 1024,
                ..Default::default()
            }
        );
        // DAX on -> default cache size
        assert_eq!(
            FsConfig::parse("tag=mytag,socket=/tmp/sock,dax=on")?,
            FsConfig {
                socket: PathBuf::from("/tmp/sock"),
                tag: "mytag".to_owned(),
                dax: true,
                cache_size: default_fsconfig_cache_size(),
                ..Default::default()
            }
        );
        assert_eq!(
            FsConfig::parse("tag=mytag,socket=/tmp/sock,dax=on,cache_size=4G")?,
            FsConfig {
                socket: PathBuf::from("/tmp/sock"),
                tag: "mytag".to_owned(),
                dax: true,
                cache_size: 4 << 30,
                ..Default::default()
            }
        );
        // Cache size without DAX is an error
        assert!(FsConfig::parse("tag=mytag,socket=/tmp/sock,dax=off,cache_size=4G").is_err());
        Ok(())
    }

    #[test]
    fn test_pmem_parsing() -> Result<()> {
        // Must always give a file and size
        assert!(PmemConfig::parse("").is_err());
        assert!(PmemConfig::parse("size=128M").is_err());
        assert_eq!(
            PmemConfig::parse("file=/tmp/pmem,size=128M")?,
            PmemConfig {
                file: PathBuf::from("/tmp/pmem"),
                size: Some(128 << 20),
                ..Default::default()
            }
        );
        assert_eq!(
            PmemConfig::parse("file=/tmp/pmem,size=128M,id=mypmem0")?,
            PmemConfig {
                file: PathBuf::from("/tmp/pmem"),
                size: Some(128 << 20),
                id: Some("mypmem0".to_owned()),
                ..Default::default()
            }
        );
        assert_eq!(
            PmemConfig::parse("file=/tmp/pmem,size=128M,iommu=on,mergeable=on,discard_writes=on")?,
            PmemConfig {
                file: PathBuf::from("/tmp/pmem"),
                size: Some(128 << 20),
                mergeable: true,
                discard_writes: true,
                iommu: true,
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn test_console_parsing() -> Result<()> {
        assert!(ConsoleConfig::parse("").is_err());
        assert!(ConsoleConfig::parse("badmode").is_err());
        assert_eq!(
            ConsoleConfig::parse("off")?,
            ConsoleConfig {
                mode: ConsoleOutputMode::Off,
                iommu: false,
                file: None,
            }
        );
        assert_eq!(
            ConsoleConfig::parse("pty")?,
            ConsoleConfig {
                mode: ConsoleOutputMode::Pty,
                iommu: false,
                file: None,
            }
        );
        assert_eq!(
            ConsoleConfig::parse("tty")?,
            ConsoleConfig {
                mode: ConsoleOutputMode::Tty,
                iommu: false,
                file: None,
            }
        );
        assert_eq!(
            ConsoleConfig::parse("null")?,
            ConsoleConfig {
                mode: ConsoleOutputMode::Null,
                iommu: false,
                file: None,
            }
        );
        assert_eq!(
            ConsoleConfig::parse("file=/tmp/console")?,
            ConsoleConfig {
                mode: ConsoleOutputMode::File,
                iommu: false,
                file: Some(PathBuf::from("/tmp/console"))
            }
        );
        assert_eq!(
            ConsoleConfig::parse("null,iommu=on")?,
            ConsoleConfig {
                mode: ConsoleOutputMode::Null,
                iommu: true,
                file: None,
            }
        );
        assert_eq!(
            ConsoleConfig::parse("file=/tmp/console,iommu=on")?,
            ConsoleConfig {
                mode: ConsoleOutputMode::File,
                iommu: true,
                file: Some(PathBuf::from("/tmp/console"))
            }
        );
        Ok(())
    }

    #[test]
    fn test_device_parsing() -> Result<()> {
        // Device must have a path provided
        assert!(DeviceConfig::parse("").is_err());
        assert_eq!(
            DeviceConfig::parse("path=/path/to/device")?,
            DeviceConfig {
                path: PathBuf::from("/path/to/device"),
                id: None,
                iommu: false
            }
        );

        assert_eq!(
            DeviceConfig::parse("path=/path/to/device,iommu=on")?,
            DeviceConfig {
                path: PathBuf::from("/path/to/device"),
                id: None,
                iommu: true
            }
        );

        assert_eq!(
            DeviceConfig::parse("path=/path/to/device,iommu=on,id=mydevice0")?,
            DeviceConfig {
                path: PathBuf::from("/path/to/device"),
                id: Some("mydevice0".to_owned()),
                iommu: true
            }
        );

        Ok(())
    }

    #[test]
    fn test_vsock_parsing() -> Result<()> {
        // socket and cid is required
        assert!(VsockConfig::parse("").is_err());
        assert_eq!(
            VsockConfig::parse("socket=/tmp/sock,cid=1")?,
            VsockConfig {
                cid: 1,
                socket: PathBuf::from("/tmp/sock"),
                iommu: false,
                id: None,
            }
        );
        assert_eq!(
            VsockConfig::parse("socket=/tmp/sock,cid=1,iommu=on")?,
            VsockConfig {
                cid: 1,
                socket: PathBuf::from("/tmp/sock"),
                iommu: true,
                id: None,
            }
        );
        Ok(())
    }

    #[test]
    fn test_config_validation() {
        let valid_config = VmConfig {
            cpus: CpusConfig {
                boot_vcpus: 1,
                max_vcpus: 1,
                ..Default::default()
            },
            memory: MemoryConfig {
                size: 536_870_912,
                mergeable: false,
                hotplug_method: HotplugMethod::Acpi,
                hotplug_size: None,
                hotplugged_size: None,
                shared: false,
                hugepages: false,
                hugepage_size: None,
                zones: None,
            },
            kernel: Some(KernelConfig {
                path: PathBuf::from("/path/to/kernel"),
            }),
            initramfs: None,
            cmdline: CmdlineConfig {
                args: String::from(""),
            },
            disks: None,
            net: None,
            rng: RngConfig {
                src: PathBuf::from("/dev/urandom"),
                iommu: false,
            },
            balloon: None,
            fs: None,
            pmem: None,
            serial: ConsoleConfig {
                file: None,
                mode: ConsoleOutputMode::Null,
                iommu: false,
            },
            console: ConsoleConfig {
                file: None,
                mode: ConsoleOutputMode::Tty,
                iommu: false,
            },
            devices: None,
            vsock: None,
            iommu: false,
            #[cfg(target_arch = "x86_64")]
            sgx_epc: None,
            numa: None,
            watchdog: false,
            #[cfg(feature = "tdx")]
            tdx: None,
        };

        assert!(valid_config.validate().is_ok());

        let mut invalid_config = valid_config.clone();
        invalid_config.serial.mode = ConsoleOutputMode::Tty;
        invalid_config.console.mode = ConsoleOutputMode::Tty;
        assert!(invalid_config.validate().is_err());

        let mut invalid_config = valid_config.clone();
        invalid_config.kernel = None;
        assert!(invalid_config.validate().is_err());

        let mut invalid_config = valid_config.clone();
        invalid_config.serial.mode = ConsoleOutputMode::File;
        invalid_config.serial.file = None;
        assert!(invalid_config.validate().is_err());

        let mut invalid_config = valid_config.clone();
        invalid_config.cpus.max_vcpus = 16;
        invalid_config.cpus.boot_vcpus = 32;
        assert!(invalid_config.validate().is_err());

        let mut invalid_config = valid_config.clone();
        invalid_config.cpus.max_vcpus = 16;
        invalid_config.cpus.boot_vcpus = 16;
        invalid_config.cpus.topology = Some(CpuTopology {
            threads_per_core: 2,
            cores_per_die: 8,
            dies_per_package: 1,
            packages: 2,
        });
        assert!(invalid_config.validate().is_err());

        let mut invalid_config = valid_config.clone();
        invalid_config.disks = Some(vec![DiskConfig {
            vhost_socket: Some("/path/to/sock".to_owned()),
            path: Some(PathBuf::from("/path/to/image")),
            ..Default::default()
        }]);
        assert!(invalid_config.validate().is_err());

        let mut invalid_config = valid_config.clone();
        invalid_config.disks = Some(vec![DiskConfig {
            vhost_user: true,
            ..Default::default()
        }]);
        assert!(invalid_config.validate().is_err());

        let mut invalid_config = valid_config.clone();
        invalid_config.disks = Some(vec![DiskConfig {
            vhost_user: true,
            vhost_socket: Some("/path/to/sock".to_owned()),
            ..Default::default()
        }]);
        assert!(invalid_config.validate().is_err());

        let mut still_valid_config = valid_config.clone();
        still_valid_config.disks = Some(vec![DiskConfig {
            vhost_user: true,
            vhost_socket: Some("/path/to/sock".to_owned()),
            ..Default::default()
        }]);
        still_valid_config.memory.shared = true;
        assert!(still_valid_config.validate().is_ok());

        let mut invalid_config = valid_config.clone();
        invalid_config.net = Some(vec![NetConfig {
            vhost_user: true,
            ..Default::default()
        }]);
        assert!(invalid_config.validate().is_err());

        let mut still_valid_config = valid_config.clone();
        still_valid_config.net = Some(vec![NetConfig {
            vhost_user: true,
            vhost_socket: Some("/path/to/sock".to_owned()),
            ..Default::default()
        }]);
        still_valid_config.memory.shared = true;
        assert!(still_valid_config.validate().is_ok());

        let mut invalid_config = valid_config.clone();
        invalid_config.net = Some(vec![NetConfig {
            fds: Some(vec![0]),
            ..Default::default()
        }]);
        assert!(invalid_config.validate().is_err());

        let mut invalid_config = valid_config.clone();
        invalid_config.fs = Some(vec![FsConfig {
            ..Default::default()
        }]);
        assert!(invalid_config.validate().is_err());

        let mut still_valid_config = valid_config.clone();
        still_valid_config.memory.shared = true;
        assert!(still_valid_config.validate().is_ok());

        let mut still_valid_config = valid_config.clone();
        still_valid_config.memory.hugepages = true;
        assert!(still_valid_config.validate().is_ok());

        let mut still_valid_config = valid_config.clone();
        still_valid_config.memory.hugepages = true;
        still_valid_config.memory.hugepage_size = Some(2 << 20);
        assert!(still_valid_config.validate().is_ok());

        let mut invalid_config = valid_config.clone();
        invalid_config.memory.hugepages = false;
        invalid_config.memory.hugepage_size = Some(2 << 20);
        assert!(invalid_config.validate().is_err());

        let mut invalid_config = valid_config;
        invalid_config.memory.hugepages = true;
        invalid_config.memory.hugepage_size = Some(3 << 20);
        assert!(invalid_config.validate().is_err());
    }
}
