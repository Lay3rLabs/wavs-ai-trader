use cosmwasm_std::Addr;

pub trait MakeAddrExt {
    fn make_addr(&self) -> Addr;
}
