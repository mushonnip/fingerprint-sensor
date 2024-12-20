#![forbid(unsafe_code)]

use byteorder::{BigEndian, ByteOrder, ReadBytesExt};
use serialport::{self, SerialPort};
use std::io::{self, Read, Write};
use std::vec::Vec;

const STARTCODE: u16 = 0xEF01;
const COMMANDPACKET: u8 = 0x1;
const ACKPACKET: u8 = 0x7;

const VERIFYPASSWORD: u8 = 0x13;
const TEMPLATECOUNT: u8 = 0x1D;
const READSYSPARAM: u8 = 0x0F;

const GETIMAGE: u8 = 0x01;
const IMAGE2TZ: u8 = 0x02;
const FINGERPRINTSEARCH: u8 = 0x04;
const REGMODEL: u8 = 0x05;
const STORE: u8 = 0x06;
const DELETE: u8 = 0x0C;

pub const OK: u8 = 0x0;
pub const NOFINGER: u8 = 0x02;
pub const IMAGEFAIL: u8 = 0x03;
pub const IMAGEMESS: u8 = 0x06;
pub const FEATUREFAIL: u8 = 0x07;
pub const INVALIDIMAGE: u8 = 0x15;
pub const HISPEEDSEARCH: u8 = 0x1B;
pub const ENROLLMISMATCH: u8 = 0x0A;
pub const BADLOCATION: u8 = 0x0B;
pub const FLASHERR: u8 = 0x18;

pub struct Device {
    _debug: bool,
    uart: Box<dyn SerialPort>,
    status_register: Option<u16>,
    system_id: Option<u16>,
    library_size: Option<u16>,
    security_level: Option<u16>,
    address: Vec<u8>,
    data_packet_size: Option<u16>,
    baudrate: Option<u16>,
    password: Vec<u8>,
    template_count: u16,
    finger_id: u16,
    confidence: u16,
}

impl Device {
    pub fn new(address: Vec<u8>, password: Vec<u8>, uart: Box<dyn SerialPort>) -> Self {
        let mut device = Self {
            _debug: false,
            uart,
            address,
            password,
            status_register: None,
            system_id: None,
            library_size: None,
            security_level: None,
            data_packet_size: None,
            baudrate: None,
            template_count: 0,
            finger_id: 0,
            confidence: 0,
        };

        if !device.verify_password() {
            panic!("Failed to find sensor, check wiring!");
        }

        if device.read_sysparam().is_err() {
            panic!("Failed to read system parameters!");
        }

        device
    }

    pub fn verify_password(&mut self) -> bool {
        let packet: Vec<u8> = std::iter::once(VERIFYPASSWORD)
            .chain(self.password.iter().cloned())
            .collect();

        if let Err(e) = self.send_packet(&packet) {
            eprintln!("Failed to send the packet: {}", e);
            return false;
        }

        let r = self.get_packet(12).unwrap_or_else(|_| vec![0; 12]);

        r[0] == OK
    }

    pub fn send_packet(&mut self, data: &[u8]) -> io::Result<()> {
        let mut packet = vec![(STARTCODE >> 8) as u8, (STARTCODE & 0xFF) as u8];

        packet.extend_from_slice(&self.address);
        packet.push(COMMANDPACKET);

        let length = data.len() + 2;
        packet.push((length >> 8) as u8);
        packet.push((length & 0xFF) as u8);

        packet.extend_from_slice(data);

        let checksum: u16 = packet[6..].iter().map(|&byte| byte as u16).sum();
        packet.push((checksum >> 8) as u8);
        packet.push((checksum & 0xFF) as u8);

        self.print_debug("send_packet length:", packet.len(), "bytes");
        self.print_debug("send_packet data:", &packet, "hex");

        self.uart.write_all(&packet)?;

        Ok(())
    }

    pub fn get_packet(&mut self, expected: usize) -> io::Result<Vec<u8>> {
        let mut res = vec![0; expected];

        self.uart.read_exact(&mut res)?;

        let start = (&res[0..2]).read_u16::<BigEndian>().unwrap();
        if start != STARTCODE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Incorrect packet data",
            ));
        }

        let addr = res[2..6].to_vec();
        if addr != self.address {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Incorrect address",
            ));
        }

        let packet_type = res[6];
        let length = (&res[7..9]).read_u16::<BigEndian>().unwrap() as usize;

        if packet_type != ACKPACKET {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Incorrect packet data",
            ));
        }

        let reply = res[9..9 + (length - 2)].to_vec();

        self.print_debug("_get_packet reply:", &reply, "hex");

        Ok(reply)
    }

    pub fn read_sysparam(&mut self) -> io::Result<u8> {
        self.send_packet(&[READSYSPARAM])?;

        let r = self.get_packet(28)?;

        if r[0] != OK {
            return Err(io::Error::new(io::ErrorKind::Other, "Command failed."));
        }

        self.status_register = Some((&r[1..3]).read_u16::<BigEndian>()?);
        self.system_id = Some((&r[3..5]).read_u16::<BigEndian>()?);
        self.library_size = Some((&r[5..7]).read_u16::<BigEndian>()?);
        self.security_level = Some((&r[7..9]).read_u16::<BigEndian>()?);
        self.address = r[9..13].to_vec();
        self.data_packet_size = Some((&r[13..15]).read_u16::<BigEndian>()?);
        self.baudrate = Some((&r[15..17]).read_u16::<BigEndian>()?);

        Ok(r[0])
    }

    pub fn count_templates(&mut self) -> io::Result<u8> {
        let _ = self.send_packet(&[TEMPLATECOUNT]);
        let r = self.get_packet(14)?;

        if r.len() >= 3 {
            self.template_count = BigEndian::read_u16(&r[1..3]);
        } else {
            self.template_count = 0;
        }

        if r[0] == OK {
            Ok(r[0])
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Command failed."))
        }
    }

    pub fn get_image(&mut self) -> io::Result<u8> {
        let _ = self.send_packet(&[GETIMAGE]);
        let r = self.get_packet(12)?;
        Ok(r[0])
    }

    pub fn image_2_tz(&mut self, slot: u8) -> io::Result<u8> {
        let _ = self.send_packet(&[IMAGE2TZ, slot]);
        let r = self.get_packet(12)?;
        Ok(r[0])
    }

    pub fn finger_search(&mut self) -> io::Result<u8> {
        if self.library_size.is_none() {
            return Err(io::Error::new(io::ErrorKind::Other, "Library size not set"));
        }
        let capacity = match self.library_size {
            Some(capacity) => capacity,
            None => return Err(io::Error::new(io::ErrorKind::Other, "Library size not set")),
        };
        let _ = self.send_packet(&[
            FINGERPRINTSEARCH,
            0x01,
            0x00,
            0x00,
            (capacity >> 8) as u8,
            (capacity & 0xFF) as u8,
        ]);
        let r = self.get_packet(16)?;
        self.finger_id = BigEndian::read_u16(&r[1..3]);
        self.confidence = BigEndian::read_u16(&r[3..5]);
        self.print_debug("finger_search packet:", &r, "hex");
        Ok(r[0])
    }

    pub fn finger_fast_search(&mut self) -> io::Result<u8> {
        let _ = self.read_sysparam();
        let capacity = match self.library_size {
            Some(capacity) => capacity,
            None => return Err(io::Error::new(io::ErrorKind::Other, "Library size not set")),
        };

        let packet = vec![
            HISPEEDSEARCH,
            0x01,
            0x00,
            0x00,
            (capacity >> 8) as u8,
            (capacity & 0xFF) as u8,
        ];

        let _ = self.send_packet(&packet);

        let r = self.get_packet(16)?;

        let finger_data = &r[1..5];
        self.finger_id = u16::from_be_bytes([finger_data[0], finger_data[1]]);
        self.confidence = u16::from_be_bytes([finger_data[2], finger_data[3]]);

        self.print_debug("finger_fast_search packet:", &r, "hex");

        Ok(r[0])
    }

    pub fn create_model(&mut self) -> io::Result<u8> {
        let _ = self.send_packet(&[REGMODEL]);
        let r = self.get_packet(12)?;
        Ok(r[0])
    }

    pub fn store_model(&mut self, location: u16, slot: u8) -> io::Result<u8> {
        self.send_packet(&[STORE, slot, (location >> 8) as u8, (location & 0xFF) as u8])?;

        let r = self.get_packet(12)?;
        Ok(r[0])
    }

    pub fn delete_model(&mut self, location: u16) -> io::Result<u8> {
        let high_byte = (location >> 8) as u8;
        let low_byte = (location & 0xFF) as u8;

        self.send_packet(&[DELETE, high_byte, low_byte, 0x00, 0x01])?;

        let r = self.get_packet(12)?;
        Ok(r[0])
    }

    fn print_debug(&self, message: &str, data: impl std::fmt::Debug, data_type: &str) {
        if self._debug {
            if data_type == "hex" {
                println!("{}: {:X?}", message, data);
            } else {
                println!("{}: {:?}", message, data);
            }
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.uart.flush().unwrap();
    }
}
