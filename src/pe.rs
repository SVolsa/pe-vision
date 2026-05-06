#![expect(dead_code)]

/// PE (Portable Executable) file format parser.
/// Hand-written for educational purposes — no third-party PE libs.

const DOS_HDR_SIZE: usize = 64;
const NT_SIG_SIZE: usize = 4;
const FILE_HDR_SIZE: usize = 20;
const SECTION_HDR_SIZE: usize = 40;
const IMPORT_DESC_SIZE: usize = 20;
const EXPORT_DIR_SIZE: usize = 40;

const DATA_DIR_NAMES: &[&str] = &[
    "Export", "Import", "Resource", "Exception",
    "Security", "Base Reloc", "Debug", "Architecture",
    "Global Ptr", "TLS", "Load Config", "Bound Import",
    "IAT", "Delay Import", "CLR Runtime", "Reserved",
];

// ---------------------------------------------------------------------------
// Data models
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct PeInfo {
    pub dos_header: DosHeader,
    pub nt_headers: NtHeaders,
    pub sections: Vec<Section>,
    pub imports: Vec<ImportDll>,
    pub exports: Option<ExportInfo>,
}

#[derive(Clone)]
pub struct DosHeader {
    pub e_magic: u16,
    pub e_lfanew: usize,
}

#[derive(Clone)]
pub struct NtHeaders {
    pub signature: u32,
    pub file_header: FileHeader,
    pub optional_header: OptionalHeader,
}

#[derive(Clone)]
pub struct FileHeader {
    pub machine: u16,
    pub num_sections: u16,
    pub timestamp: u32,
    pub size_of_optional_header: u16,
    pub characteristics: u16,
}

#[derive(Clone)]
pub enum OptionalHeader {
    Pe32(OptHdr32),
    Pe32Plus(OptHdr64),
}

#[derive(Clone)]
pub struct OptHdr32 {
    pub entry_point: u32,
    pub image_base: u32,
    pub section_align: u32,
    pub file_align: u32,
    pub image_size: u32,
    pub subsystem: u16,
    pub data_dir: Vec<DataDirEntry>,
}

#[derive(Clone)]
pub struct OptHdr64 {
    pub entry_point: u32,
    pub image_base: u64,
    pub section_align: u32,
    pub file_align: u32,
    pub image_size: u32,
    pub subsystem: u16,
    pub data_dir: Vec<DataDirEntry>,
}

#[derive(Clone)]
pub struct DataDirEntry {
    pub name: &'static str,
    pub rva: u32,
    pub size: u32,
}

#[derive(Clone)]
pub struct Section {
    pub name: String,
    pub virt_size: u32,
    pub virt_addr: u32,
    pub raw_size: u32,
    pub raw_offset: u32,
    pub characteristics: u32,
}

#[derive(Clone)]
pub struct ImportDll {
    pub name: String,
    pub funcs: Vec<String>,
}

#[derive(Clone)]
pub struct ExportInfo {
    pub dll_name: String,
    pub base: u32,
    pub funcs: Vec<ExportFunc>,
}

#[derive(Clone)]
pub struct ExportFunc {
    pub ordinal: u32,
    pub name: Option<String>,
    pub rva: u32,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

macro_rules! read_at {
    ($data:expr, $offset:expr, $n:literal, $ty:ty) => {{
        let off = $offset;
        let slice = $data.get(off..off + $n).ok_or_else(|| {
            format!("read_u{} overflow at 0x{:X}", $n * 8, off)
        })?;
        Ok(<$ty>::from_le_bytes(slice.try_into().unwrap()))
    }};
}

fn read_u16(data: &[u8], off: usize) -> Result<u16, String> {
    read_at!(data, off, 2, u16)
}
fn read_u32(data: &[u8], off: usize) -> Result<u32, String> {
    read_at!(data, off, 4, u32)
}
fn read_u64(data: &[u8], off: usize) -> Result<u64, String> {
    read_at!(data, off, 8, u64)
}

fn read_cstr(data: &[u8], off: usize) -> Result<String, String> {
    let tail = data.get(off..).ok_or("string offset out of range")?;
    let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
    Ok(String::from_utf8_lossy(&tail[..end]).to_string())
}

// ---------------------------------------------------------------------------
// Main parse entry
// ---------------------------------------------------------------------------

pub fn parse_pe(data: &[u8]) -> Result<PeInfo, String> {
    if data.len() < DOS_HDR_SIZE {
        return Err("File too small for DOS header".into());
    }
    let magic = read_u16(data, 0)?;
    if magic != 0x5A4D {
        return Err("Not a PE file — missing MZ signature".into());
    }

    let e_lfanew = read_u32(data, 0x3C)? as usize;
    if e_lfanew + NT_SIG_SIZE + FILE_HDR_SIZE > data.len() {
        return Err("e_lfanew points beyond file".into());
    }

    let sig = read_u32(data, e_lfanew)?;
    if sig != 0x0000_4550 {
        return Err("Not a PE file — missing PE signature".into());
    }

    let dos = DosHeader { e_magic: magic, e_lfanew };
    let nt = parse_nt_headers(data, e_lfanew)?;
    let sections = parse_sections(data, e_lfanew, nt.file_header.num_sections)?;

    // Data-directory index 0 = export, 1 = import
    let imports = parse_imports(data, &sections, &nt.optional_header)?;
    let exports = parse_exports(data, &sections, &nt.optional_header)?;

    Ok(PeInfo { dos_header: dos, nt_headers: nt, sections, imports, exports })
}

// ---------------------------------------------------------------------------
// NT Headers
// ---------------------------------------------------------------------------

fn parse_nt_headers(data: &[u8], e_lfanew: usize) -> Result<NtHeaders, String> {
    let off = e_lfanew + NT_SIG_SIZE; // skip signature
    let fh = parse_file_header(data, off)?;

    let opt_off = off + FILE_HDR_SIZE;
    let oh = parse_optional_header(data, opt_off, fh.size_of_optional_header)?;

    Ok(NtHeaders { signature: 0x0000_4550, file_header: fh, optional_header: oh })
}

fn parse_file_header(data: &[u8], off: usize) -> Result<FileHeader, String> {
    Ok(FileHeader {
        machine:                 read_u16(data, off)?,
        num_sections:            read_u16(data, off + 2)?,
        timestamp:               read_u32(data, off + 4)?,
        size_of_optional_header: read_u16(data, off + 16)?,
        characteristics:         read_u16(data, off + 18)?,
    })
}

fn parse_optional_header(
    data: &[u8],
    off: usize,
    size_of_oh: u16,
) -> Result<OptionalHeader, String> {
    if size_of_oh < 2 {
        return Err("Optional header too small".into());
    }
    let magic = read_u16(data, off)?;
    match magic {
        0x10B => parse_opt32(data, off),
        0x20B => parse_opt64(data, off),
        _ => Err(format!("Unknown optional header magic: 0x{:04X}", magic)),
    }
}

fn parse_opt32(data: &[u8], off: usize) -> Result<OptionalHeader, String> {
    // off+0: Magic (u16, consumed above)
    // off+2..: standard COFF fields
    let ep   = read_u32(data, off + 16)?;
    let ib   = read_u32(data, off + 28)?;
    let sa   = read_u32(data, off + 32)?;
    let fa   = read_u32(data, off + 36)?;
    let isz  = read_u32(data, off + 56)?;
    let subs = read_u16(data, off + 68)?;
    let dd_count = read_u32(data, off + 92)?;

    let mut dd = Vec::new();
    let dd_off = off + 96;
    for i in 0..dd_count.min(16) {
        let e = dd_off + (i as usize) * 8;
        let rva = read_u32(data, e)?;
        let sz  = read_u32(data, e + 4)?;
        let name = DATA_DIR_NAMES.get(i as usize).copied().unwrap_or("Unknown");
        dd.push(DataDirEntry { name, rva, size: sz });
    }

    Ok(OptionalHeader::Pe32(OptHdr32 {
        entry_point: ep, image_base: ib,
        section_align: sa, file_align: fa,
        image_size: isz, subsystem: subs,
        data_dir: dd,
    }))
}

fn parse_opt64(data: &[u8], off: usize) -> Result<OptionalHeader, String> {
    let ep   = read_u32(data, off + 16)?;
    let ib   = read_u64(data, off + 24)?;
    let sa   = read_u32(data, off + 32)?;
    let fa   = read_u32(data, off + 36)?;
    let isz  = read_u32(data, off + 56)?;
    let subs = read_u16(data, off + 68)?;
    let dd_count = read_u32(data, off + 108)?;

    let mut dd = Vec::new();
    let dd_off = off + 112;
    for i in 0..dd_count.min(16) {
        let e = dd_off + (i as usize) * 8;
        let rva = read_u32(data, e)?;
        let sz  = read_u32(data, e + 4)?;
        let name = DATA_DIR_NAMES.get(i as usize).copied().unwrap_or("Unknown");
        dd.push(DataDirEntry { name, rva, size: sz });
    }

    Ok(OptionalHeader::Pe32Plus(OptHdr64 {
        entry_point: ep, image_base: ib,
        section_align: sa, file_align: fa,
        image_size: isz, subsystem: subs,
        data_dir: dd,
    }))
}

// ---------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------

fn parse_sections(data: &[u8], e_lfanew: usize, count: u16) -> Result<Vec<Section>, String> {
    // section table starts right after optional header
    let fh_off = e_lfanew + NT_SIG_SIZE;
    let oh_size = read_u16(data, fh_off + 16)?;
    let sec_off = fh_off + FILE_HDR_SIZE + oh_size as usize;

    let mut sections = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let off = sec_off + i * SECTION_HDR_SIZE;
        let raw_name = data.get(off..off + 8).unwrap_or(&[0u8; 8]);
        let name_end = raw_name.iter().position(|&b| b == 0).unwrap_or(8);
        let name = String::from_utf8_lossy(&raw_name[..name_end]).to_string();

        sections.push(Section {
            name,
            virt_size:       read_u32(data, off + 8)?,
            virt_addr:       read_u32(data, off + 12)?,
            raw_size:        read_u32(data, off + 16)?,
            raw_offset:      read_u32(data, off + 20)?,
            characteristics: read_u32(data, off + 36)?,
        });
    }
    Ok(sections)
}

// ---------------------------------------------------------------------------
// RVA → file offset
// ---------------------------------------------------------------------------

fn rva_to_offset(rva: u32, sections: &[Section]) -> Option<usize> {
    for s in sections {
        if rva >= s.virt_addr && rva < s.virt_addr + s.virt_size {
            return Some((s.raw_offset + (rva - s.virt_addr)) as usize);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Import table
// ---------------------------------------------------------------------------

fn parse_imports(
    data: &[u8],
    sections: &[Section],
    oh: &OptionalHeader,
) -> Result<Vec<ImportDll>, String> {
    let dirs = match oh {
        OptionalHeader::Pe32(h) => &h.data_dir,
        OptionalHeader::Pe32Plus(h) => &h.data_dir,
    };
    let import_dir = match dirs.get(1) {
        Some(d) if d.rva != 0 => d,
        _ => return Ok(Vec::new()),
    };

    let base_off = match rva_to_offset(import_dir.rva, sections) {
        Some(o) => o,
        None => return Ok(Vec::new()),
    };

    let is_64 = matches!(oh, OptionalHeader::Pe32Plus(_));
    let mut dlls = Vec::new();

    for i in 0.. {
        let off = base_off + i * IMPORT_DESC_SIZE;
        if off + IMPORT_DESC_SIZE > data.len() {
            break;
        }
        let name_rva = read_u32(data, off + 12)?;
        let orig_thunk = read_u32(data, off)?;
        if name_rva == 0 && orig_thunk == 0 {
            break; // sentinel
        }

        let name_off = match rva_to_offset(name_rva, sections) {
            Some(o) => o,
            None => continue,
        };
        let dll_name = read_cstr(data, name_off)?;

        let thunk_rva = if orig_thunk != 0 { orig_thunk } else { read_u32(data, off + 16)? };
        let thunk_off = match rva_to_offset(thunk_rva, sections) {
            Some(o) => o,
            None => {
                dlls.push(ImportDll { name: dll_name, funcs: Vec::new() });
                continue;
            }
        };

        let mut funcs = Vec::new();
        for j in 0.. {
            let toff = thunk_off + j * if is_64 { 8 } else { 4 };
            if is_64 {
                let val = read_u64(data, toff)?;
                if val == 0 { break; }
                if val & (1u64 << 63) != 0 {
                    funcs.push(format!("ordinal({})", val & 0xFFFF));
                } else {
                    push_import_name(data, val as u32, sections, &mut funcs);
                }
            } else {
                let val = read_u32(data, toff)?;
                if val == 0 { break; }
                if val & 0x8000_0000 != 0 {
                    funcs.push(format!("ordinal({})", val & 0xFFFF));
                } else {
                    push_import_name(data, val, sections, &mut funcs);
                }
            }
        }
        dlls.push(ImportDll { name: dll_name, funcs });
    }
    Ok(dlls)
}

fn push_import_name(data: &[u8], rva: u32, sections: &[Section], funcs: &mut Vec<String>) {
    let off = match rva_to_offset(rva, sections) {
        Some(o) => o,
        None => { funcs.push(format!("0x{:08X}", rva)); return; }
    };
    // skip hint u16
    match read_cstr(data, off + 2) {
        Ok(n) => funcs.push(n),
        Err(_) => funcs.push(format!("0x{:08X}", rva)),
    }
}

// ---------------------------------------------------------------------------
// Export table
// ---------------------------------------------------------------------------

fn parse_exports(
    data: &[u8],
    sections: &[Section],
    oh: &OptionalHeader,
) -> Result<Option<ExportInfo>, String> {
    let dirs = match oh {
        OptionalHeader::Pe32(h) => &h.data_dir,
        OptionalHeader::Pe32Plus(h) => &h.data_dir,
    };
    let exp_dir = match dirs.get(0) {
        Some(d) if d.rva != 0 => d,
        _ => return Ok(None),
    };

    let base_off = match rva_to_offset(exp_dir.rva, sections) {
        Some(o) => o,
        None => return Ok(None),
    };

    let name_rva   = read_u32(data, base_off + 12)?;
    let base       = read_u32(data, base_off + 16)?;
    let num_funcs  = read_u32(data, base_off + 20)?;
    let num_names  = read_u32(data, base_off + 24)?;
    let addr_fns   = read_u32(data, base_off + 28)?;
    let addr_names = read_u32(data, base_off + 32)?;
    let addr_ords  = read_u32(data, base_off + 36)?;

    let dll_name = match rva_to_offset(name_rva, sections) {
        Some(off) => read_cstr(data, off).unwrap_or_else(|_| "?".to_string()),
        None => "?".to_string(),
    };

    let fn_off = match rva_to_offset(addr_fns, sections) {
        Some(o) => o,
        None => return Ok(None),
    };
    let name_off = match rva_to_offset(addr_names, sections) {
        Some(o) => o,
        None => 0,
    };
    let ord_off = match rva_to_offset(addr_ords, sections) {
        Some(o) => o,
        None => 0,
    };

    let mut funcs = Vec::with_capacity(num_funcs as usize);
    for i in 0..num_funcs as usize {
        let ea = read_u32(data, fn_off + i * 4).unwrap_or(0);
        funcs.push(ExportFunc { ordinal: base + i as u32, name: None, rva: ea });
    }

    for i in 0..num_names.min(num_funcs) as usize {
        if name_off == 0 || ord_off == 0 { break; }
        let name_rva_val = read_u32(data, name_off + i * 4).unwrap_or(0);
        let ord = read_u16(data, ord_off + i * 2).unwrap_or(0) as usize;
        if let Some(off) = rva_to_offset(name_rva_val, sections) {
            if let Ok(n) = read_cstr(data, off) {
                if let Some(f) = funcs.get_mut(ord) {
                    f.name = Some(n);
                }
            }
        }
    }

    Ok(Some(ExportInfo { dll_name, base, funcs }))
}
