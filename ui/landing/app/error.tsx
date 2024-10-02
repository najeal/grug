"use client";

import { useRouter } from "next/navigation";
import { useEffect } from "react";

interface ErrorPageProps {
  error: Error & { digest?: string };
  reset: () => void;
}

// biome-ignore lint/suspicious/noShadowRestrictedNames: Error Page
export default function Error({ error }: ErrorPageProps) {
  const { push } = useRouter();
  useEffect(() => {
    console.error(error);
  }, [error]);

  return (
    <div className="flex flex-1 flex-col justify-center items-center z-10">
      <h2>Something went wrong!</h2>
      <button type="button" onClick={() => push("/")}>
        Go back to home
      </button>
    </div>
  );
}
