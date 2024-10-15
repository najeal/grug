import type {
  Address,
  Chain,
  Client,
  Proposal,
  ProposalId,
  Signer,
  Transport,
} from "@leftcurve/types";
import { queryWasmSmart } from "~/actions/public/queryWasmSmart";

export type SafeAccountGetProposalsParameters = {
  address: Address;
  startAfter?: ProposalId;
  limit?: number;
  height?: number;
};

export type SafeAccountGetProposalsReturnType = Promise<Record<ProposalId, Proposal>>;

/**
 * Get the proposals of a specific account
 * @param parameters
 * @param parameters.startAfter The proposal to start after.
 * @param parameters.limit The number of proposals to return.
 * @param parameters.height The height at which to query for the proposals
 * @returns The proposals of the account.
 */
export async function safeAccountGetProposals<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: SafeAccountGetProposalsParameters,
): SafeAccountGetProposalsReturnType {
  const { address, limit, startAfter, height = 0 } = parameters || {};
  const msg = {
    proposals: { startAfter, limit },
  };

  return await queryWasmSmart(client, { contract: address, msg, height });
}