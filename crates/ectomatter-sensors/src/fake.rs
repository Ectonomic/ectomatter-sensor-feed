//! Made-up computers to list and read, for tests: here and in Ectomatter's,
//! whose screen and Settings tests read them through the Sensor Feed plugin's
//! library.

use std::path::{Path, PathBuf};

use crate::list::{Nv, Nvidia, Probe, Val};

/// A made-up NVIDIA driver: the cards it has, each with what it reports.
pub struct FakeNvidia(pub Vec<(String, Probe)>);

impl Nvidia for FakeNvidia {
    fn probe(&mut self, pci: &str) -> Option<Probe> {
        self.0.iter().find(|(p, _)| p == pci).map(|(_, p)| p.clone())
    }
    // (an RTX 4070's readings)
    fn read(&mut self, pci: &str, what: Nv) -> Option<Val> {
        self.probe(pci).filter(|p| p.has.contains(&what))?;
        Some(match what {
            Nv::Temp => Val::One(43.0),
            Nv::Load => Val::One(6.0),
            Nv::Clock => Val::One(1882.0),
            Nv::MemClock => Val::One(7001.0),
            Nv::Vram => Val::Of(4.8, 31.8),
            Nv::Power => Val::One(73.4),
            Nv::Fan => Val::One(0.0),
        })
    }
}

/// A made-up computer: an AMD processor with two chiplets and four cores of
/// two threads, an MSI board's NCT6687 (a temperature reading 193°, fans, voltages), two
/// sticks of DDR5, two drives, an NVIDIA card, an Intel Arc card and the processor's own
/// Radeon graphics, an Aquantia Ethernet chip and a MediaTek Wi-Fi card.
pub fn computer(dir: &Path) {
    // (each link leads where it would from its own folder, so the computer
    // reads the same wherever its folder is seen from: moved, or in a sandbox)
    let link = |to: &Path, at: &Path| {
        let from = at.parent().unwrap();
        let common = to.ancestors().find(|a| from.starts_with(a)).unwrap();
        let ups = from.strip_prefix(common).unwrap().components().count();
        let mut rel = PathBuf::new();
        for _ in 0..ups {
            rel.push("..");
        }
        rel.push(to.strip_prefix(common).unwrap());
        std::os::unix::fs::symlink(rel, at).unwrap();
    };
    let w = |p: &str, s: &str| {
        let f = dir.join(p);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, format!("{s}\n")).unwrap();
    };
    let n = std::cell::Cell::new(0);
    let hw = |name: &str, dev: &str, files: &[(&str, &str)]| {
        let h = dir.join(format!("sys/class/hwmon/hwmon{}", n.get()));
        n.set(n.get() + 1);
        std::fs::create_dir_all(&h).unwrap();
        std::fs::write(h.join("name"), format!("{name}\n")).unwrap();
        let d = dir.join(dev);
        std::fs::create_dir_all(&d).unwrap();
        link(&d, &h.join("device"));
        for (f, v) in files {
            std::fs::write(h.join(f), format!("{v}\n")).unwrap();
        }
    };
    hw("k10temp", "sys/devices/pci0000:00/0000:00:18.3", &[("temp1_input", "66000"), ("temp1_label", "Tctl"), ("temp3_input", "58000"), ("temp3_label", "Tccd1"), ("temp4_input", "54000"), ("temp4_label", "Tccd2")]);
    hw("nct6687", "sys/devices/platform/nct6687.2592", &[
        ("temp1_input", "65000"), ("temp1_label", "CPU"), ("temp2_input", "50000"), ("temp2_label", "System"), ("temp7_input", "193000"), ("temp7_label", "M2_1"),
        ("fan1_input", "734"), ("fan1_label", "CPU Fan"), ("fan2_input", "2836"), ("fan2_label", "Pump Fan"),
        ("in0_input", "12000"), ("in0_label", "+12V"), ("in1_input", "4980"), ("in1_label", "+5V"), ("in10_input", "1550"), ("in10_label", "Voltage #2"),
    ]);
    hw("spd5118", "sys/devices/pci0000:00/0000:00:14.0/i2c-1/1-0051", &[("temp1_input", "49000"), ("temp1_enable", "1")]);
    hw("spd5118", "sys/devices/pci0000:00/0000:00:14.0/i2c-1/1-0050", &[("temp1_input", "47000"), ("temp1_enable", "1")]);
    w("sys/devices/pci0000:00/0000:00:01.2/0000:02:00.0/class", "0x010802");
    w("sys/devices/pci0000:00/0000:00:01.2/0000:02:00.0/nvme/nvme0/model", "Samsung SSD 990 PRO 2TB           ");
    w("sys/devices/pci0000:00/0000:00:01.2/0000:02:00.0/nvme/nvme0/serial", "S0EXAMPLE00001M     ");
    hw("nvme", "sys/devices/pci0000:00/0000:00:01.2/0000:02:00.0/nvme/nvme0", &[("temp1_input", "47850"), ("temp1_label", "Composite"), ("temp2_input", "53850"), ("temp2_label", "Sensor 1")]);
    w("sys/devices/pci0000:00/0000:00:02.1/0000:0f:00.0/class", "0x010802");
    w("sys/devices/pci0000:00/0000:00:02.1/0000:0f:00.0/nvme/nvme3/model", "WDS100T3X0C-00SJG0");
    w("sys/devices/pci0000:00/0000:00:02.1/0000:0f:00.0/nvme/nvme3/serial", "100000000002");
    hw("nvme", "sys/devices/pci0000:00/0000:00:02.1/0000:0f:00.0/nvme/nvme3", &[("temp1_input", "42850"), ("temp1_label", "Composite")]);
    // the Ethernet chip names its hwmon after its network: enp6s0
    for (f, v) in [("class", "0x020000"), ("vendor", "0x1d6a"), ("device", "0x07b1")] {
        w(&format!("sys/devices/pci0000:00/0000:00:02.1/0000:06:00.0/{f}"), v);
    }
    hw("enp6s0", "sys/devices/pci0000:00/0000:00:02.1/0000:06:00.0", &[("temp1_input", "50000"), ("temp1_label", "PHY Temperature"), ("temp2_input", "50000"), ("temp2_label", "MAC Temperature")]);
    for (f, v) in [("class", "0x028000"), ("vendor", "0x14c3"), ("device", "0x0608")] {
        w(&format!("sys/devices/pci0000:00/0000:00:02.1/0000:0b:00.0/{f}"), v);
    }
    hw("mt7921_phy0", "sys/devices/pci0000:00/0000:00:02.1/0000:0b:00.0/ieee80211/phy0", &[("temp1_input", "48000")]);
    // the cards
    let card = |path: &str, v: &str, d: &str| {
        for (f, s) in [("class", "0x030000"), ("vendor", v), ("device", d)] {
            w(&format!("sys/devices/{path}/{f}"), s);
        }
        let at = dir.join("sys/bus/pci/devices");
        std::fs::create_dir_all(&at).unwrap();
        link(&dir.join("sys/devices").join(path), &at.join(path.rsplit('/').next().unwrap()));
        // (a driver running it)
        std::fs::create_dir_all(dir.join("sys/bus/pci/drivers/gpu")).unwrap();
        link(&dir.join("sys/bus/pci/drivers/gpu"), &dir.join("sys/devices").join(path).join("driver"));
    };
    card("pci0000:00/0000:00:01.1/0000:01:00.0", "0x10de", "0x2786");
    let arc = "pci0000:00/0000:00:02.2/0000:16:00.0";
    card(arc, "0x8086", "0x56a1");
    w(&format!("sys/devices/{arc}/tile0/gt0/gtidle/idle_residency_ms"), "28005947");
    w(&format!("sys/devices/{arc}/tile0/gt0/freq0/act_freq"), "1200");
    hw("xe", &format!("sys/devices/{arc}"), &[
        ("temp2_input", "49000"), ("temp2_label", "pkg"), ("temp3_input", "48000"), ("temp3_label", "vram"), ("temp6_input", "48000"), ("temp6_label", "vram_ch_0"),
        ("fan1_input", "1255"), ("energy1_input", "1000000"), ("energy1_label", "card"), ("energy2_input", "500000"), ("energy2_label", "pkg"),
    ]);
    let radeon = "pci0000:00/0000:00:08.1/0000:18:00.0";
    card(radeon, "0x1002", "0x164e");
    w(&format!("sys/devices/{radeon}/gpu_busy_percent"), "6");
    w(&format!("sys/devices/{radeon}/mem_info_vram_used"), "75743232");
    w(&format!("sys/devices/{radeon}/mem_info_vram_total"), "2147483648");
    hw("amdgpu", &format!("sys/devices/{radeon}"), &[
        ("temp1_input", "54000"), ("temp1_label", "edge"), ("freq1_input", "2200000000"), ("freq1_label", "sclk"), ("power1_input", "100000"), ("power1_label", "PPT"),
        ("in0_input", "1340"), ("in0_label", "vddgfx"), ("in1_input", "1200"), ("in1_label", "vddnb"),
    ]);
    w("usr/share/hwdata/pci.ids", "1002  Advanced Micro Devices, Inc. [AMD/ATI]\n\t164e  Raphael [Radeon Graphics]\n10de  NVIDIA Corporation\n\t2786  AD104 [GeForce RTX 4070]\n14c3  MEDIATEK Corp.\n\t0608  MT7921 802.11ax PCI Express Wireless Network Adapter\n1d6a  Aquantia Corp.\n\t07b1  AQtion AQC107 NBase-T/IEEE 802.3bz Ethernet Controller [Atlantic 10G]\n8086  Intel Corporation\n\t56a1  DG2 [Arc A750]");
    // the processor: four cores of two threads (0 and 4 share the first)
    for t in 0..8 {
        w(&format!("sys/devices/system/cpu/cpu{t}/topology/physical_package_id"), "0");
        w(&format!("sys/devices/system/cpu/cpu{t}/topology/core_id"), &(t % 4).to_string());
        w(&format!("sys/devices/system/cpu/cpu{t}/cpufreq/scaling_cur_freq"), &(if t < 4 { 5_400_000 + t * 100_000 } else { 624_194 }).to_string());
    }
    w("proc/cpuinfo", "processor\t: 0\nmodel name\t: AMD Ryzen 7 7800X3D 8-Core Processor\ncpu cores\t: 4");
    w("proc/meminfo", "MemTotal:       63119752 kB\nMemFree:        1 kB\nMemAvailable:   28100000 kB");
    w("sys/class/dmi/id/board_vendor", "Micro-Star International Co., Ltd.");
    w("sys/class/dmi/id/board_name", "MAG B650 TOMAHAWK WIFI (MS-7D75)");
    w("sys/class/dmi/id/chassis_type", "3");
}

pub fn nvidia() -> FakeNvidia {
    FakeNvidia(vec![("0000:01:00.0".into(), Probe { has: vec![Nv::Temp, Nv::Load, Nv::Clock, Nv::Vram, Nv::MemClock, Nv::Power, Nv::Fan], vram: 31.8 })])
}

pub fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ectomatter-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

