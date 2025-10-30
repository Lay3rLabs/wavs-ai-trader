use layer_climb::prelude::CosmosAddr;

#[derive(Clone)]
pub enum AnyAddr {
    CosmWasm(cosmwasm_std::Addr),
    ClimbCosmos(layer_climb::prelude::CosmosAddr),
    ClimbAddress(layer_climb::prelude::Address),
}

impl std::fmt::Display for AnyAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnyAddr::CosmWasm(a) => write!(f, "{}", a),
            AnyAddr::ClimbCosmos(a) => write!(f, "{}", a),
            AnyAddr::ClimbAddress(a) => write!(f, "{}", a),
        }
    }
}

impl std::fmt::Debug for AnyAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnyAddr::CosmWasm(a) => write!(f, "{:?}", a),
            AnyAddr::ClimbCosmos(a) => write!(f, "{:?}", a),
            AnyAddr::ClimbAddress(a) => write!(f, "{:?}", a),
        }
    }
}

impl From<cosmwasm_std::Addr> for AnyAddr {
    fn from(addr: cosmwasm_std::Addr) -> Self {
        AnyAddr::CosmWasm(addr)
    }
}

impl From<&cosmwasm_std::Addr> for AnyAddr {
    fn from(addr: &cosmwasm_std::Addr) -> Self {
        AnyAddr::CosmWasm(addr.clone())
    }
}

impl From<layer_climb::prelude::CosmosAddr> for AnyAddr {
    fn from(addr: layer_climb::prelude::CosmosAddr) -> Self {
        AnyAddr::ClimbCosmos(addr)
    }
}

impl From<&layer_climb::prelude::CosmosAddr> for AnyAddr {
    fn from(addr: &layer_climb::prelude::CosmosAddr) -> Self {
        AnyAddr::ClimbCosmos(addr.clone())
    }
}

impl From<layer_climb::prelude::Address> for AnyAddr {
    fn from(addr: layer_climb::prelude::Address) -> Self {
        AnyAddr::ClimbAddress(addr)
    }
}

impl From<&layer_climb::prelude::Address> for AnyAddr {
    fn from(addr: &layer_climb::prelude::Address) -> Self {
        AnyAddr::ClimbAddress(addr.clone())
    }
}

impl From<AnyAddr> for cosmwasm_std::Addr {
    fn from(addr: AnyAddr) -> Self {
        match addr {
            AnyAddr::CosmWasm(a) => a,
            AnyAddr::ClimbCosmos(a) => a.into(),
            AnyAddr::ClimbAddress(a) => CosmosAddr::try_from(a).unwrap().into(),
        }
    }
}

impl From<&AnyAddr> for cosmwasm_std::Addr {
    fn from(addr: &AnyAddr) -> Self {
        match addr {
            AnyAddr::CosmWasm(a) => a.clone(),
            AnyAddr::ClimbCosmos(a) => a.clone().into(),
            AnyAddr::ClimbAddress(a) => CosmosAddr::try_from(a.clone()).unwrap().into(),
        }
    }
}

impl From<AnyAddr> for layer_climb::prelude::CosmosAddr {
    fn from(addr: AnyAddr) -> Self {
        match addr {
            AnyAddr::CosmWasm(a) => CosmosAddr::new_str(a.as_str(), None).unwrap(),
            AnyAddr::ClimbCosmos(a) => a,
            AnyAddr::ClimbAddress(a) => CosmosAddr::try_from(a).unwrap(),
        }
    }
}

impl From<&AnyAddr> for layer_climb::prelude::CosmosAddr {
    fn from(addr: &AnyAddr) -> Self {
        match addr {
            AnyAddr::CosmWasm(a) => CosmosAddr::new_str(a.as_str(), None).unwrap(),
            AnyAddr::ClimbCosmos(a) => a.clone(),
            AnyAddr::ClimbAddress(a) => CosmosAddr::try_from(a.clone()).unwrap(),
        }
    }
}

impl From<AnyAddr> for layer_climb::prelude::Address {
    fn from(addr: AnyAddr) -> Self {
        match addr {
            AnyAddr::CosmWasm(a) => layer_climb::prelude::CosmosAddr::new_str(a.as_str(), None)
                .unwrap()
                .into(),
            AnyAddr::ClimbCosmos(a) => a.into(),
            AnyAddr::ClimbAddress(a) => a,
        }
    }
}

impl From<&AnyAddr> for layer_climb::prelude::Address {
    fn from(addr: &AnyAddr) -> Self {
        match addr {
            AnyAddr::CosmWasm(a) => layer_climb::prelude::CosmosAddr::new_str(a.as_str(), None)
                .unwrap()
                .into(),
            AnyAddr::ClimbCosmos(a) => a.clone().into(),
            AnyAddr::ClimbAddress(a) => a.clone(),
        }
    }
}
