use bitvec::prelude::*;
use libc::{
    J1939_IDLE_ADDR, J1939_NO_ADDR, J1939_NO_PGN, J1939_PGN_ADDRESS_CLAIMED,
    J1939_PGN_ADDRESS_COMMANDED, J1939_PGN_MAX, J1939_PGN_PDU1_MAX, J1939_PGN_REQUEST,
};

/// Parameter group number defined in "SAE J1939/21 – Data Link Layer"
///
/// # Format
/// The kernel stack expects the following format for a PGN
/// bits 0 - 7: Protocol Data Unit (PDU) specific
/// bits 8 - 15: PDU format
/// bit 16: Data page
/// bit 17: Reserved
///
/// # Note
///
/// The kernel stack ignores the PDU specific part of the PGN if it is in PDU1 format
/// (destination specific message). The destination address must be set separately through
/// [J1939SockAddr](crate::addr::J1939SockAddr).
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pgn(u32);

impl Pgn {
    /// Indicates the absence of a PGN, see https://www.kernel.org/doc/html/next/networking/j1939.html
    pub const NO_PGN: Self = Self::from(J1939_NO_PGN);
    /// Address Claimed (PGN 0xEE00)
    pub const ADDRESS_CLAIMED: Self = Self::from(J1939_PGN_ADDRESS_CLAIMED);
    /// PGN Request (PGN 0xEA00)
    pub const PGN_REQUEST: Self = Self::from(J1939_PGN_REQUEST);
    /// PGN Address Commanded (PGN 0xFED8)
    pub const ADDRESS_COMMANDED: Self = Self::from(J1939_PGN_ADDRESS_COMMANDED);

    /// Creates a new PGN from a `u32`. The value is truncated to 18 bit of a valid PGN.
    pub const fn from(pgn: u32) -> Self {
        Pgn(pgn & J1939_PGN_MAX)
    }

    /// Creates a new PGN from the data page, PDU format, and PDU specific parts.
    pub fn new(data_page: bool, pdu_format: u8, pdu_specific: u8) -> Self {
        let mut pgn = Self(0);
        pgn.set_data_page(data_page);
        pgn.set_pdu_format(pdu_format);
        pgn.set_pdu_specific(pdu_specific);
        pgn
    }

    /// Return if the data page bit is set
    pub fn data_page(&self) -> bool {
        self.0.view_bits::<Lsb0>()[16]
    }

    /// Sets the data page bit
    pub fn set_data_page(&mut self, value: bool) {
        self.0.view_bits_mut::<Lsb0>().replace(16, value);
    }

    /// Returns the PDU format part
    pub fn pdu_format(&self) -> u8 {
        self.0.view_bits::<Lsb0>()[8..16].load_le()
    }

    /// Sets the PDU format
    pub fn set_pdu_format(&mut self, value: u8) {
        self.0.view_bits_mut::<Lsb0>()[8..16].store_le(value);
    }

    /// Returns the PDU specific part
    pub fn pdu_specific(&self) -> u8 {
        self.0.view_bits::<Lsb0>()[0..8].load_le()
    }

    /// Sets the PDU specific part
    pub fn set_pdu_specific(&mut self, value: u8) {
        self.0.view_bits_mut::<Lsb0>()[0..8].store_le(value);
    }

    /// Returns `true` if the PGN has the PDU1 format
    pub fn is_pdu1(&self) -> bool {
        self.pdu_format() <= 239
    }

    /// Return `true` if the PGN has the PDU2 format
    pub fn is_pdu2(&self) -> bool {
        self.pdu_format() >= 240
    }

    /// Returns the PGN as a 3 Byte array in little endian byte order.
    pub fn to_le_bytes(&self) -> [u8; 3] {
        let mut buf: [u8; 3] = [0, 0, 0];
        buf.copy_from_slice(&self.0.to_le_bytes()[0..3]);
        buf
    }

    /// Create a PGN from a `u8` array (expects litte endian byte order).
    pub fn from_le_bytes(buf: &[u8; 3]) -> Self {
        let mut pgn_buf: [u8; 4] = [0, 0, 0, 0];
        pgn_buf[0..3].copy_from_slice(&buf[0..3]);
        Pgn::from(u32::from_le_bytes(pgn_buf))
    }
}

impl From<Pgn> for u32 {
    fn from(pgn: Pgn) -> Self {
        if pgn.is_pdu1() {
            // The Linux kernel J1939 stacks expects the PGN to have PDU2 format.
            // The destination address for PDU1 format is set as separate parameter
            // in the Berkely socket interface.
            // See https://www.kernel.org/doc/html/next/networking/j1939.html
            pgn.0 & J1939_PGN_PDU1_MAX
        } else {
            pgn.0
        }
    }
}

/// Address of a Control Function on the J1939 network
/// defined in "SAE J1939/21 – Data Link Layer"
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Addr(u8);

impl Addr {
    /// No destination specific address (0xFF). It is also used to
    /// for broadcast messages to all devices on the network.
    pub const NO_ADDR: Self = Self(J1939_NO_ADDR);

    /// Broadcast address (0xFF), which is used to send a message to all devices
    /// on the network. It is equal to [Self::NO_ADDR], and just provided for
    /// improved readability.
    pub const BROADCAST: Self = Self(J1939_NO_ADDR);

    /// Idle address (0xFE)
    pub const IDLE_ADDR: Self = Self(J1939_IDLE_ADDR);
}

impl From<Addr> for u8 {
    fn from(addr: Addr) -> Self {
        addr.0
    }
}

impl From<u8> for Addr {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

/// J1939 NAME identifier defined in "SAE J1939/81 – Network Management"
///
/// The J1939 NAME is a 64-bit Id to uniquely identify a control function (ECU)
/// on the network. The NAME consists of:
///
/// * Bits 0-20: Identity number
/// * Bits 21-31: Manufacturer code
/// * Bits 32-34: ECU instance
/// * Bits 35-39: Function instance
/// * Bits 40-47: Function
/// * Bit 48: Reserved
/// * Bits 49-55: Vehicle system
/// * Bits 56-59: Vehicle system instance
/// * Bits 60-62: Industry group
/// * Bit 63: Abitrary address capable
///
/// For more details, see ISO11783-3
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name(BitArr!(for 64));

impl Name {
    /// Special NAME used in [J1939Addr](crate::addr::J1939Addr) to indicate that no NAME was set.
    pub const NO_NAME: Self = Self(BitArray::ZERO);

    /// Return the NAME as a byte array in little-endian byte order.
    pub fn to_le_bytes(&self) -> [u8; 8] {
        self.0.load::<u64>().to_le_bytes()
    }
}

impl From<u64> for Name {
    fn from(value: u64) -> Self {
        let mut name = Self::default();
        name.0.store(value);
        name
    }
}

impl From<Name> for u64 {
    fn from(name: Name) -> Self {
        name.0.load_le()
    }
}

impl Default for Name {
    fn default() -> Self {
        Self(BitArray::ZERO)
    }
}

impl Name {
    /// Return if a control function is abritray address capable.
    /// If true then the control function supports dynamic addressing.
    pub fn arbitrary_address_capable(&self) -> bool {
        self.0[63]
    }

    /// Sets the abritrary address capable field.
    pub fn set_arbitrary_address_capable(&mut self, val: bool) {
        self.0.replace(63, val);
    }

    /// Returns the industry group of the control function.
    pub fn industry_group(&self) -> u8 {
        self.0[60..63].load_le()
    }

    /// Sets the industry group field. The  value is truncated to the allowed range (3 bit).
    pub fn set_industry_group(&mut self, value: u8) {
        self.0[60..63].store_le(value);
    }

    /// Returns the vehicle system instance of the control function.
    pub fn vehicle_system_instance(&self) -> u8 {
        self.0[56..60].load_le()
    }

    /// Sets the vehicle system instance field. The value is truncated to the allowed range (4 bit).
    pub fn set_vehicle_system_instance(&mut self, value: u8) {
        self.0[56..60].store_le(value)
    }

    /// Returns the vehicle system of the control function
    pub fn vehicle_system(&self) -> u8 {
        self.0[49..56].load_le()
    }

    /// Sets the vehicle system field. The value is truncated to the allowed range (7 bit).
    pub fn set_vehicle_system(&mut self, value: u8) {
        self.0[49..56].store_le(value)
    }

    /// Return the function of the control function.
    pub fn function(&self) -> u8 {
        self.0[40..48].load_le()
    }

    /// Sets the function field. The value is truncated to the allowed range (8 bit).
    pub fn set_function(&mut self, value: u8) {
        self.0[40..48].store_le(value)
    }

    /// Returns the function instance of the control function.
    pub fn function_instance(&self) -> u8 {
        self.0[35..40].load_le()
    }

    /// Sets the function instance field. The value is truncated to the allowed range (5 bit).
    pub fn set_function_instance(&mut self, value: u8) {
        self.0[35..40].store_le(value)
    }

    /// Returns the ECU instance of the control function.
    pub fn ecu_instance(&self) -> u8 {
        self.0[32..35].load_le()
    }

    /// Sets the ECU instance field. The value is truncated to the allowed range (3 bit).
    pub fn set_ecu_instance(&mut self, value: u8) {
        self.0[32..35].store_le(value)
    }

    /// Returns the manufacturer code of the control function.
    pub fn manufacturer_code(&self) -> u16 {
        self.0[21..32].load_le()
    }

    /// Sets the manufacturer code field. The value is truncated to the allowed range (11 bit).
    pub fn set_manufacturer_code(&mut self, value: u16) {
        self.0[21..32].store_le(value)
    }

    /// Returns the identity number of the control function.
    pub fn identity_number(&self) -> u32 {
        self.0[0..21].load_le()
    }

    /// Sets the identity number field. The value is truncated to the allowed range (21 bit)
    pub fn set_identity_number(&mut self, value: u32) {
        self.0[0..21].store_le(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pgn_from_parts() {
        let pgn = Pgn::new(true, 0xF0, 0x04);
        assert!(pgn.data_page());
        assert_eq!(pgn.pdu_format(), 0xF0);
        assert_eq!(pgn.pdu_specific(), 0x04);
        assert_eq!(Into::<u32>::into(pgn), 0x01F004);
    }

    #[test]
    fn test_create_pgn_from_u32() {
        let pgn = Pgn::from(0x01F004);
        assert!(pgn.data_page());
        assert_eq!(pgn.pdu_format(), 0xF0);
        assert_eq!(pgn.pdu_specific(), 0x04);
    }

    #[test]
    fn test_create_name_from_raw() {
        let name = Name::from(0x9704033501000004);
        assert!(name.arbitrary_address_capable());
        assert_eq!(name.industry_group(), 1);
        assert_eq!(name.vehicle_system_instance(), 7);
        assert_eq!(name.vehicle_system(), 2);
        assert_eq!(name.function(), 3);
        assert_eq!(name.function_instance(), 6);
        assert_eq!(name.manufacturer_code(), 8);
        assert_eq!(name.ecu_instance(), 5);
        assert_eq!(name.identity_number(), 4);
    }

    #[test]
    fn access_arbitray_address_capable() {
        let mut name = Name::from(0x9704033501000004);
        assert!(name.arbitrary_address_capable());
        name.set_arbitrary_address_capable(false);
        assert!(!name.arbitrary_address_capable());
    }

    #[test]
    fn access_industry_group() {
        let mut name = Name::from(0x9704033501000004);
        assert_eq!(name.industry_group(), 1);
        name.set_industry_group(0);
        assert_eq!(name.industry_group(), 0);
        name.set_industry_group(7);
        assert_eq!(name.industry_group(), 7);
        name.set_industry_group(1);
        assert_eq!(u64::from(name), 0x9704033501000004);
    }

    #[test]
    fn access_vehicle_system_instance() {
        let mut name = Name::from(0x9704033501000004);
        assert_eq!(name.vehicle_system_instance(), 7);
        name.set_vehicle_system_instance(0x0F);
        assert_eq!(name.vehicle_system_instance(), 0x0F);
        name.set_vehicle_system_instance(0);
        assert_eq!(name.vehicle_system_instance(), 0);
        name.set_vehicle_system_instance(7);
        assert_eq!(u64::from(name), 0x9704033501000004);
    }

    #[test]
    fn access_vehicle_system() {
        let mut name = Name::from(0x9704033501000004);
        assert_eq!(name.vehicle_system(), 2);
        name.set_vehicle_system(0x7F);
        assert_eq!(name.vehicle_system(), 0x7F);
        name.set_vehicle_system(0);
        assert_eq!(name.vehicle_system(), 0);
        name.set_vehicle_system(2);
        assert_eq!(u64::from(name), 0x9704033501000004);
    }

    #[test]
    fn access_function() {
        let mut name = Name::from(0x9704033501000004);
        assert_eq!(name.function(), 3);
        name.set_function(0xFF);
        assert_eq!(name.function(), 0xFF);
        name.set_function(0);
        assert_eq!(name.function(), 0);
        name.set_function(3);
        assert_eq!(u64::from(name), 0x9704033501000004);
    }

    #[test]
    fn access_function_instance() {
        let mut name = Name::from(0x9704033501000004);
        assert_eq!(name.function_instance(), 6);
        name.set_function_instance(0x1F);
        assert_eq!(name.function_instance(), 0x1F);
        name.set_function_instance(0);
        assert_eq!(name.function_instance(), 0);
        name.set_function_instance(6);
        assert_eq!(u64::from(name), 0x9704033501000004);
    }

    #[test]
    fn access_ecu_instance() {
        let mut name = Name::from(0x9704033501000004);
        assert_eq!(name.ecu_instance(), 5);
        name.set_ecu_instance(0x07);
        assert_eq!(name.ecu_instance(), 0x07);
        name.set_ecu_instance(0);
        assert_eq!(name.ecu_instance(), 0);
        name.set_ecu_instance(5);
        assert_eq!(u64::from(name), 0x9704033501000004);
    }

    #[test]
    fn access_manufacturer_code() {
        let mut name = Name::from(0x9704033501000004);
        assert_eq!(name.manufacturer_code(), 8);
        name.set_manufacturer_code(0x07FF);
        assert_eq!(name.manufacturer_code(), 0x07FF);
        name.set_manufacturer_code(0);
        assert_eq!(name.manufacturer_code(), 0);
        name.set_manufacturer_code(8);
        assert_eq!(u64::from(name), 0x9704033501000004);
    }

    #[test]
    fn access_identity_number() {
        let mut name = Name::from(0x9704033501000004);
        assert_eq!(name.identity_number(), 4);
        name.set_identity_number(0x001FFFFF);
        assert_eq!(name.identity_number(), 0x001FFFFF);
        name.set_identity_number(0);
        assert_eq!(name.identity_number(), 0);
        name.set_identity_number(4);
        assert_eq!(u64::from(name), 0x9704033501000004);
    }
}
