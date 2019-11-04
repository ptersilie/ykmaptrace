use object::Object;
use phdrs::{objects};

use hwtracer::{Trace};
use std::{borrow, env, fs};
use std::collections::HashMap;

pub struct MapTrace {
    phdr_offset: u64,
    labels: Option<HashMap<u64, String>>
}

impl MapTrace {

    pub fn new() -> MapTrace {
        let phdr_offset = get_phdr_offset();
        let labels = match extract_labels() {
            Ok(l) => Some(l),
            Err(_) => None
        };
        MapTrace {
            phdr_offset,
            labels
        }
    }

    pub fn annotate_trace(&self, trace: Box<dyn Trace>) -> Option<Vec<(u64, &String)>> {
        if !self.labels.is_some() {
            return None;
        }
        let labels = self.labels.as_ref().unwrap();
        let mut annotrace = Vec::new();
        for b in trace.iter_blocks() {
            match b {
                Ok(block) => {
                    let addr = block.start_vaddr() - self.phdr_offset;
                    match labels.get(&addr) {
                        Some(l) => annotrace.push((addr, l)),
                        None => { }
                    };
                },
                Err(e) => println!("{}", e)
            }
        }
        Some(annotrace)
    }

}

fn get_phdr_offset() -> u64 {
    let o = objects();
    (&o[0]).addr() as u64
}


fn extract_labels() -> Result<HashMap<u64, String>, gimli::Error> {
    // Load executable
    let pathb = match env::current_exe() {
        Ok(p) => p,
        Err(e) => panic!(e)
    };
    let file = fs::File::open(&pathb.as_path()).unwrap();
    let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };
    let object = object::File::parse(&*mmap).unwrap();
    let endian = if object.is_little_endian() {
        gimli::RunTimeEndian::Little
    } else {
        gimli::RunTimeEndian::Big
    };

    // Extract labels

    let mut labels = HashMap::new();

    let loader = |id: gimli::SectionId| -> Result<borrow::Cow<[u8]>, gimli::Error> {
        Ok(object
            .section_data_by_name(id.name())
            .unwrap_or(borrow::Cow::Borrowed(&[][..])))
    };
    let sup_loader = |_| Ok(borrow::Cow::Borrowed(&[][..]));
    let dwarf_cow = gimli::Dwarf::load(&loader, &sup_loader)?;
    let borrow_section: &dyn for<'a> Fn(&'a borrow::Cow<[u8]>,)
        -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
        &|section| gimli::EndianSlice::new(&*section, endian);
    let dwarf = dwarf_cow.borrow(&borrow_section);
    let mut iter = dwarf.units();
    while let Some(header) = iter.next()? {
        let unit = dwarf.unit(header)?;
        let mut entries = unit.entries();
        while let Some((_, entry)) = entries.next_dfs()? {
            if entry.tag() == gimli::DW_TAG_subprogram {
                if let Some(name) = entry.attr_value(gimli::DW_AT_linkage_name)? {
                    let s = name.string_value(&dwarf.debug_str).unwrap();
                    if let Some(lowpc) = entry.attr_value(gimli::DW_AT_low_pc)? {
                        let addr = match lowpc {
                            gimli::AttributeValue::Addr(v) => v as u64,
                            _ => panic!("This should be an Addr")
                        };
                        //println!("SUBPROG: {}", addr);
                        labels.insert(addr, s.to_string()?.to_string());
                    }
                }
            }
            else if entry.tag() == gimli::DW_TAG_label {
                if let Some(name) = entry.attr_value(gimli::DW_AT_name)? {
                    if let Some(es) = name.string_value(&dwarf.debug_str) {
                        let s = es.to_string()?;
                        if s.starts_with("__YK_") {
                            if let Some(lowpc) = entry.attr_value(gimli::DW_AT_low_pc)? {
                                let addr = match lowpc {
                                    gimli::AttributeValue::Addr(v) => v as u64,
                                    _ => panic!("This should be an Addr")
                                };
                                labels.insert(addr, s.to_string());
                                //println!("{:x?} {}", addr, s.to_string());
                            }
                            else {
                                // XXX What to do with labels without pow_pc
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(labels)
}
