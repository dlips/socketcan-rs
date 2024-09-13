use crate::j1939::protocol::{Addr, Name, Pgn};
use libc::{j1939_filter, J1939_PGN_MAX, J1939_PGN_PDU1_MAX};

/// Represents a receive filter for the [`J1939Socket`]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct J1939Filter(j1939_filter);

impl J1939Filter {
    /// Creates a new filter from partial filters.
    pub fn new(name_filter: NameFilter, pgn_filter: PgnFilter, addr_filter: AddrFilter) -> Self {
        let filter = j1939_filter {
            name: name_filter.name,
            name_mask: name_filter.mask,
            pgn: pgn_filter.pgn,
            pgn_mask: pgn_filter.mask,
            addr: addr_filter.addr,
            addr_mask: addr_filter.mask,
        };
        J1939Filter(filter)
    }

    /// Creates a new filter from unchecked raw values.
    pub fn new_raw(
        name: u64,
        name_mask: u64,
        pgn: u32,
        pgn_mask: u32,
        addr: u8,
        addr_mask: u8,
    ) -> Self {
        let filter = j1939_filter {
            name,
            name_mask,
            pgn: pgn & J1939_PGN_MAX,
            pgn_mask: pgn_mask & J1939_PGN_MAX,
            addr,
            addr_mask,
        };
        J1939Filter(filter)
    }
}

impl AsRef<j1939_filter> for J1939Filter {
    fn as_ref(&self) -> &j1939_filter {
        &self.0
    }
}

impl From<J1939Filter> for j1939_filter {
    fn from(filter: J1939Filter) -> Self {
        filter.0
    }
}

/// A mask for the J1939 NAME filter.
#[derive(Debug, Clone, Copy)]
pub enum NameFilterMask {
    /// Partial filter bitmask
    Partial(u64),
    /// Full NAME matching
    Full,
}

impl From<NameFilterMask> for u64 {
    fn from(mask: NameFilterMask) -> Self {
        match mask {
            NameFilterMask::Full => 0xFFFFFFFFFFFFFFFF,
            NameFilterMask::Partial(mask) => mask,
        }
    }
}

/// A J1939 NAME filter.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct NameFilter {
    name: u64,
    mask: u64,
}

impl NameFilter {
    /// Create new [NameFilter]
    pub fn new(name: Name, mask: NameFilterMask) -> Self {
        Self {
            name: name.into(),
            mask: mask.into(),
        }
    }
}

impl From<NameFilter> for J1939Filter {
    fn from(name_filter: NameFilter) -> Self {
        let filter = j1939_filter {
            name: name_filter.name,
            name_mask: name_filter.mask,
            pgn: 0,
            pgn_mask: 0,
            addr: 0,
            addr_mask: 0,
        };
        J1939Filter(filter)
    }
}

/// A mask for a J1939 PGN filter.
#[derive(Debug, Clone, Copy)]
pub enum PgnFilterMask {
    /// Match bitmask corresponding to PDU1 format
    Pdu1,
    /// Partial filter bitmask
    Partial(u32),
    /// Full PGN matching
    Full,
}

impl From<PgnFilterMask> for u32 {
    fn from(mask: PgnFilterMask) -> Self {
        match mask {
            PgnFilterMask::Pdu1 => J1939_PGN_PDU1_MAX,
            PgnFilterMask::Partial(mask) => mask & J1939_PGN_MAX,
            PgnFilterMask::Full => J1939_PGN_MAX,
        }
    }
}

/// A J1939 PGN filter.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct PgnFilter {
    pgn: u32,
    mask: u32,
}

impl PgnFilter {
    /// Create new [PgnFilter]
    pub fn new(pgn: Pgn, mask: PgnFilterMask) -> Self {
        Self {
            pgn: pgn.into(),
            mask: mask.into(),
        }
    }
}

impl From<PgnFilter> for J1939Filter {
    fn from(pgn_filter: PgnFilter) -> Self {
        let filter = j1939_filter {
            name: 0,
            name_mask: 0,
            pgn: pgn_filter.pgn,
            pgn_mask: pgn_filter.mask,
            addr: 0,
            addr_mask: 0,
        };
        J1939Filter(filter)
    }
}

/// A mask for a J1939 control function address filter.
#[derive(Debug, Clone, Copy)]
pub enum AddrFilterMask {
    /// Partial address filter bitmask
    Partial(u8),
    /// Full address matching
    Full,
}

impl From<AddrFilterMask> for u8 {
    fn from(mask: AddrFilterMask) -> Self {
        match mask {
            AddrFilterMask::Full => 0xFF,
            AddrFilterMask::Partial(mask) => mask,
        }
    }
}

/// A J1939 control function address filter.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct AddrFilter {
    addr: u8,
    mask: u8,
}

impl AddrFilter {
    /// Create a new [AddrFilter]
    pub fn new(addr: Addr, mask: AddrFilterMask) -> Self {
        Self {
            addr: addr.into(),
            mask: mask.into(),
        }
    }
}

impl From<AddrFilter> for J1939Filter {
    fn from(addr_filter: AddrFilter) -> Self {
        let filter = j1939_filter {
            name: 0,
            name_mask: 0,
            pgn: 0,
            pgn_mask: 0,
            addr: addr_filter.addr,
            addr_mask: addr_filter.mask,
        };
        J1939Filter(filter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::j1939::protocol::{Name, Pgn};
    use libc::j1939_filter;

    #[test]
    fn create_name_filter() {
        let name = Name::from(0x9704033501000004);
        let filter: J1939Filter = NameFilter::new(name, NameFilterMask::Full).into();
        let filter_ref: &j1939_filter = filter.as_ref();
        assert_eq!(filter_ref.name, 0x9704033501000004);
        assert_eq!(filter_ref.name_mask, 0xFFFFFFFFFFFFFFFF);
    }

    #[test]
    fn create_pgn_filter() {
        let pgn = Pgn::from(0x2100);
        let filter: J1939Filter = PgnFilter::new(pgn, PgnFilterMask::Pdu1).into();
        let filter_ref: &j1939_filter = filter.as_ref();
        assert_eq!(filter_ref.pgn, 0x2100);
        assert_eq!(filter_ref.pgn_mask, 0x3FF00);

        let pgn = Pgn::from(0x21FF);
        let filter: J1939Filter = PgnFilter::new(pgn, PgnFilterMask::Full).into();
        let filter_ref: &j1939_filter = filter.as_ref();
        assert_eq!(filter_ref.pgn, 0x2100);
        assert_eq!(filter_ref.pgn_mask, 0x3FFFF);
    }

    #[test]
    fn create_addr_filter() {
        let addr = Addr::from(0x12);
        let filter: J1939Filter = AddrFilter::new(addr, AddrFilterMask::Partial(0x0F)).into();
        let filter_ref: &j1939_filter = filter.as_ref();
        assert_eq!(filter_ref.addr, 0x12);
        assert_eq!(filter_ref.addr_mask, 0x0F);

        let addr = Addr::from(0x12);
        let filter: J1939Filter = AddrFilter::new(addr, AddrFilterMask::Full).into();
        let filter_ref: &j1939_filter = filter.as_ref();
        assert_eq!(filter_ref.addr, 0x12);
        assert_eq!(filter_ref.addr_mask, 0xFF);
    }
}
