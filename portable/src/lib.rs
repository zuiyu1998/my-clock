#![no_std]

pub mod ui;
pub mod button;
pub mod datetime;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
