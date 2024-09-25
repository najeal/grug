import { createMemoryStorage, createStorage } from "@leftcurve/connect-kit";

import type { Storage } from "@leftcurve/types";
import type { Dispatch, SetStateAction } from "react";
import { useQuery } from "../query";

export type UseStorageOptions<T = undefined> = {
  initialValue?: T | (() => T);
  storage?: Storage;
};
export function useStorage<T = undefined>(
  key: string,
  options: UseStorageOptions<T> = {},
): [T, Dispatch<SetStateAction<T>>] {
  const { initialValue: _initialValue_, storage: _storage_ } = options;

  const storage = (() => {
    if (_storage_) return _storage_;
    return createStorage({ key: "grustorage", storage: createMemoryStorage() });
  })();

  const initialValue = (() => {
    if (typeof _initialValue_ !== "function") return _initialValue_ as T;
    return (_initialValue_ as () => T)();
  })();

  const { data, refetch, ...rest } = useQuery<T, Error, T, string[]>({
    queryKey: [key],
    queryFn: () => (storage.getItem(key) as T) ?? initialValue,
    initialData: initialValue,
  });

  const setValue = (valOrFunc: T | ((t: T) => void)) => {
    const newState = (() => {
      if (typeof valOrFunc !== "function") return valOrFunc as T;
      return (valOrFunc as (prevState: T) => T)(data as T);
    })();

    storage.setItem(key, newState);
    refetch();
  };

  return [data as T, setValue];
}

export default useStorage;
