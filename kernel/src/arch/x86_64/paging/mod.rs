pub mod page_table;

pub fn init(mem_map: &mut limine::response::MemoryMapResponse) {
    page_table::init(mem_map);
}
