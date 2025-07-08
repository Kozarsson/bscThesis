pub mod frost;

pub trait Settings {
    fn system_size(&self) -> u16;
    fn threshold(&self) -> u16;
}
