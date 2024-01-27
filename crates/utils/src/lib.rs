use {
    anyhow::{bail, ensure},
    cw_std::{hash, to_json, Addr, Binary, Coins, Config, GenesisState, Hash, Message},
    serde::ser::Serialize,
    std::{fs::File, io::Read, path::Path},
};

pub enum AdminOption {
    SetToAddr(Addr),
    SetToSelf,
    SetToNone,
}

/// Helper for building genesis state. See the examples folder of this repository
/// for an example.
#[derive(Default)]
pub struct GenesisBuilder {
    cfg:        Option<Config>,
    code_msgs:  Vec<Message>,
    other_msgs: Vec<Message>,
}

impl GenesisBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn store_code(&mut self, path: impl AsRef<Path>) -> anyhow::Result<Hash> {
        // read Wasm byte code from file
        let mut file = File::open(path)?;
        let mut wasm_byte_code = vec![];
        file.read_to_end(&mut wasm_byte_code)?;

        // compute hash
        let code_hash = hash(&wasm_byte_code);

        // push the message into queue
        self.code_msgs.push(Message::StoreCode {
            wasm_byte_code: wasm_byte_code.into(),
        });

        Ok(code_hash)
    }

    pub fn instantiate<M: Serialize>(
        &mut self,
        code_hash: Hash,
        msg:       M,
        salt:      Binary,
        funds:     Coins,
        admin:     AdminOption,
    ) -> anyhow::Result<Addr> {
        // derive the contract address
        // note that for now we use an all-zero address as the message sender
        // during genesis. this design may change.
        let contract = Addr::compute(&Addr::mock(0), &code_hash, &salt);

        // decide contract admin
        let admin = match admin {
            AdminOption::SetToAddr(addr) => Some(addr),
            AdminOption::SetToSelf => Some(contract.clone()),
            AdminOption::SetToNone => None,
        };

        // push the message into queue
        self.other_msgs.push(Message::Instantiate {
            code_hash,
            msg: to_json(&msg)?,
            salt,
            funds,
            admin,
        });

        Ok(contract)
    }

    pub fn store_code_and_instantiate<M: Serialize>(
        &mut self,
        path:  impl AsRef<Path>,
        msg:   M,
        salt:  Binary,
        funds: Coins,
        admin: AdminOption,
    ) -> anyhow::Result<Addr> {
        let code_hash = self.store_code(path)?;
        self.instantiate(code_hash, msg, salt, funds, admin)
    }

    pub fn execute<M: Serialize>(
        &mut self,
        contract: Addr,
        msg:      M,
        funds:    Coins,
    ) -> anyhow::Result<()> {
        self.other_msgs.push(Message::Execute {
            contract,
            msg: to_json(&msg)?,
            funds,
        });

        Ok(())
    }

    pub fn set_config(&mut self, cfg: Config) -> anyhow::Result<()> {
        ensure!(self.cfg.is_none(), "Config is set twice. Something is probably wrong in your workflow");

        self.cfg = Some(cfg);

        Ok(())
    }

    pub fn finalize(mut self) -> anyhow::Result<GenesisState> {
        // a config must have been provided
        let Some(config) = self.cfg.take() else {
            bail!("Config is not set");
        };

        // ensure that store code messages are in front of all other msgs
        let mut msgs = self.code_msgs;
        msgs.extend(self.other_msgs);

        Ok(GenesisState { config, msgs })
    }
}
