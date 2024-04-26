use {
    crate::{
        from_json_slice, to_json_vec, BankQueryMsg, BankQueryResponse, Batch, Context,
        GenericResult, Hash, IbcClientUpdateMsg, IbcClientVerifyMsg, Json, Response, StdError,
        Storage, SubMsgResult, TransferMsg, Tx,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{de::DeserializeOwned, ser::Serialize},
};

// ------------------------------------ db -------------------------------------

/// Represents a database that our blockchain operates over.
///
/// The database should follow [ADR-065](https://github.com/cosmos/cosmos-sdk/blob/main/docs/architecture/adr-065-store-v2.md).
/// That is, it should contain two components:
/// - **state commitment**, a _Merklized_ KV store that stores _hashed_ keys and
///   _hashed_ values;
/// - **state storage**, a _flat_ KV store that stores _raw_ keys and _raw_ values.
///
/// The `state_commitment` and `state_storage` methods should return an _owned_
/// instance of the storage object (see the required `'static` lifetime). This
/// is required by the Wasm runtime. Additionally, storage object should be
/// _read only_. The host should write changes to an in-memory caching layer
/// (e.g. using `cw_db::CacheStore`) and then use the `flush` and `commit`
/// methods to persist them.
///
/// The two mutable methods `flush` and `commit` take an immutable reference
/// of self (`&self`) instead of a mutable one. This is needed for multithreading.
/// For this reason, the implementation should use the [interior mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)
/// pattern such as `Arc<RwLock<T>>`.
///
/// We ship three implementations of `Db`:
/// - for use in production nodes, a physical DB based on RocksDB;
/// - for testing, a temporary, in-memory KV store;
/// - for tests that utilize mainnet data ("mainnet forking"), a DB that pulls
///   data from a remote archive node.
pub trait Db {
    type Error: ToString;

    /// Type of the Merkle proof. The DB can choose any Merkle tree scheme.
    type Proof: Serialize + DeserializeOwned;

    /// Return the state commitment as an owned, read-only, `Storage` object.
    /// This should be a _Merklized_ KV store that stores _hashed_ keys and _hashed_ values.
    fn state_commitment(&self) -> impl Storage + Clone + 'static;

    /// Return the state storage as an owned, read-only, `Storage` object.
    /// This should be a _flat_ KV store that stores _raw_ keys and _raw_ values.
    fn state_storage(&self, version: Option<u64>) -> impl Storage + Clone + 'static;

    /// Return the most recent version that has been committed.
    /// `None` if not a single version has been committed.
    fn latest_version(&self) -> Option<u64>;

    /// Return the Merkle root hash at the specified version.
    /// If version is unspecified, return that of the latest committed version.
    /// `None` if the Merkle tree is empty at that version, or if that version
    /// has been pruned (we can't differentiate these two situations).
    fn root_hash(&self, version: Option<u64>) -> Result<Option<Hash>, Self::Error>;

    /// Generate Merkle proof of the given key at the given version.
    /// If version is unspecified, use the latest version.
    /// If the key exists at that version, the returned value should be a Merkle
    /// _membership_ proof; otherwise, it should be a _non-membership_ proof.
    fn prove(&self, key: &[u8], version: Option<u64>) -> Result<Self::Proof, Self::Error>;

    /// Accept a batch ops (an op is either a DB insertion or a deletion), keep
    /// them in the memory, but do not persist to disk yet; also, increment the
    /// version.
    ///
    /// This is typically invoked in the ABCI `FinalizeBlock` call.
    fn flush_but_not_commit(&self, batch: Batch) -> Result<(u64, Option<Hash>), Self::Error>;

    /// Persist pending data added in the `flush` method to disk.
    ///
    /// This is typically invoked in the ABCI `Commit` call.
    fn commit(&self) -> Result<(), Self::Error>;

    /// Flush and commit in one go.
    ///
    /// This is typically only invoked in the ABCI `InitChain` call.
    fn flush_and_commit(&self, batch: Batch) -> Result<(u64, Option<Hash>), Self::Error> {
        let (new_version, root_hash) = self.flush_but_not_commit(batch)?;
        self.commit()?;
        Ok((new_version, root_hash))
    }
}

// ------------------------------------ vm -------------------------------------

/// Represents a virtual machine that can execute programs.
pub trait Vm: Sized {
    type Error: From<StdError> + ToString;

    type Storage;
    type Querier;

    /// The type of programs intended to be run in this VM.
    ///
    /// It must be hashable and serializable with Borsh, so that it can be
    /// stored in such a mapping: hash(program) => program.
    type Program: std::hash::Hash + BorshSerialize + BorshDeserialize;

    /// Create an instance of the VM given a storage, a querier, and a guest
    /// program.
    fn build_instance(
        storage: Self::Storage,
        querier: Self::Querier,
        program: Self::Program,
    ) -> Result<Self, Self::Error>;

    /// Call a function that takes exactly 0 input parameter (other than the
    /// context) and returns exactly 1 output.
    fn call_in_0_out_1(&mut self, name: &str, ctx: &Context) -> Result<Vec<u8>, Self::Error>;

    /// Call a function that takes exactly 1 input parameter (other than the
    /// context) and returns exactly 1 output.
    fn call_in_1_out_1(
        &mut self,
        name: &str,
        ctx: &Context,
        param1: impl AsRef<[u8]>,
    ) -> Result<Vec<u8>, Self::Error>;

    /// Call a function that takes exactly 2 input parameters (other than the
    /// context) and returns exactly 1 output.
    fn call_in_2_out_1(
        &mut self,
        name: &str,
        ctx: &Context,
        param1: impl AsRef<[u8]>,
        param2: impl AsRef<[u8]>,
    ) -> Result<Vec<u8>, Self::Error>;

    fn call_instantiate(
        &mut self,
        ctx: &Context,
        msg: &Json,
    ) -> Result<GenericResult<Response>, Self::Error> {
        let res_bytes = self.call_in_1_out_1("instantiate", ctx, to_json_vec(msg)?)?;
        Ok(from_json_slice(res_bytes)?)
    }

    fn call_execute(
        &mut self,
        ctx: &Context,
        msg: &Json,
    ) -> Result<GenericResult<Response>, Self::Error> {
        let res_bytes = self.call_in_1_out_1("execute", ctx, to_json_vec(msg)?)?;
        Ok(from_json_slice(res_bytes)?)
    }

    fn call_query(
        &mut self,
        ctx: &Context,
        msg: &Json,
    ) -> Result<GenericResult<Json>, Self::Error> {
        let res_bytes = self.call_in_1_out_1("query", ctx, to_json_vec(msg)?)?;
        Ok(from_json_slice(res_bytes)?)
    }

    fn call_migrate(
        &mut self,
        ctx: &Context,
        msg: &Json,
    ) -> Result<GenericResult<Response>, Self::Error> {
        let res_bytes = self.call_in_1_out_1("migrate", ctx, to_json_vec(msg)?)?;
        Ok(from_json_slice(res_bytes)?)
    }

    fn call_reply(
        &mut self,
        ctx: &Context,
        msg: &Json,
        events: &SubMsgResult,
    ) -> Result<GenericResult<Response>, Self::Error> {
        let res_bytes = self.call_in_2_out_1(
            "reply",
            ctx,
            to_json_vec(msg)?,
            to_json_vec(events)?,
        )?;
        Ok(from_json_slice(res_bytes)?)
    }

    fn call_receive(&mut self, ctx: &Context) -> Result<GenericResult<Response>, Self::Error> {
        let res_bytes = self.call_in_0_out_1("receive", ctx)?;
        Ok(from_json_slice(res_bytes)?)
    }

    fn call_before_block(&mut self, ctx: &Context) -> Result<GenericResult<Response>, Self::Error> {
        let res_bytes = self.call_in_0_out_1("before_block", ctx)?;
        Ok(from_json_slice(res_bytes)?)
    }

    fn call_after_block(&mut self, ctx: &Context) -> Result<GenericResult<Response>, Self::Error> {
        let res_bytes = self.call_in_0_out_1("after_block", ctx)?;
        Ok(from_json_slice(res_bytes)?)
    }

    fn call_before_tx(
        &mut self,
        ctx: &Context,
        tx: &Tx,
    ) -> Result<GenericResult<Response>, Self::Error> {
        let res_bytes = self.call_in_1_out_1("before_tx", ctx, to_json_vec(tx)?)?;
        Ok(from_json_slice(res_bytes)?)
    }

    fn call_after_tx(
        &mut self,
        ctx: &Context,
        tx: &Tx,
    ) -> Result<GenericResult<Response>, Self::Error> {
        let res_bytes = self.call_in_1_out_1("after_tx", ctx, to_json_vec(tx)?)?;
        Ok(from_json_slice(res_bytes)?)
    }

    fn call_bank_transfer(
        &mut self,
        ctx: &Context,
        msg: &TransferMsg,
    ) -> Result<GenericResult<Response>, Self::Error> {
        let res_bytes = self.call_in_1_out_1("bank_transfer", ctx, to_json_vec(msg)?)?;
        Ok(from_json_slice(res_bytes)?)
    }

    fn call_bank_query(
        &mut self,
        ctx: &Context,
        msg: &BankQueryMsg,
    ) -> Result<GenericResult<BankQueryResponse>, Self::Error> {
        let res_bytes = self.call_in_1_out_1("bank_query", ctx, to_json_vec(msg)?)?;
        Ok(from_json_slice(res_bytes)?)
    }

    fn call_ibc_client_create(
        &mut self,
        ctx: &Context,
        client_state: &Json,
        consensus_state: &Json,
    ) -> Result<GenericResult<Response>, Self::Error> {
        let res_bytes = self.call_in_2_out_1(
            "ibc_client_create",
            ctx,
            to_json_vec(client_state)?,
            to_json_vec(consensus_state)?,
        )?;
        Ok(from_json_slice(res_bytes)?)
    }

    fn call_ibc_client_update(
        &mut self,
        ctx: &Context,
        msg: &IbcClientUpdateMsg,
    ) -> Result<GenericResult<Response>, Self::Error> {
        let res_bytes = self.call_in_1_out_1("ibc_client_update", ctx, to_json_vec(msg)?)?;
        Ok(from_json_slice(res_bytes)?)
    }

    fn call_ibc_client_verify(
        &mut self,
        ctx: &Context,
        msg: &IbcClientVerifyMsg,
    ) -> Result<GenericResult<()>, Self::Error> {
        let res_bytes = self.call_in_1_out_1("ibc_client_verify", ctx, to_json_vec(msg)?)?;
        Ok(from_json_slice(res_bytes)?)
    }
}
