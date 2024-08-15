import { deserialize, serialize } from "@leftcurve/encoding";

import type {
  Account,
  Chain,
  Client,
  SimulateRequest,
  SimulateResponse,
  Transport,
} from "@leftcurve/types";

export type SimulateParameters = {
  simulate: SimulateRequest;
  height?: number;
};

export type SimulateReturnType = Promise<SimulateResponse>;

export async function simulate<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(client: Client<Transport, chain, account>, parameters: SimulateParameters): SimulateReturnType {
  const { simulate, height = 0 } = parameters;
  const res = await client.query("/simulate", serialize(simulate), height, false);
  return deserialize<SimulateResponse>(res.value);
}
