"use client";

export function Hero() {
  return (
    <div className="relative overflow-hidden border-b border-zinc-200 bg-gradient-to-b from-zinc-50 to-white dark:border-zinc-800 dark:from-zinc-950 dark:to-black">
      <div className="container mx-auto px-4 py-16 sm:px-6 sm:py-24 lg:px-8">
        <div className="mx-auto max-w-3xl text-center">
          <div className="mb-6 inline-flex items-center rounded-full border border-zinc-200 bg-white px-4 py-1.5 text-sm font-medium text-zinc-900 shadow-sm dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-50">
            <span className="mr-2 h-2 w-2 rounded-full bg-blue-500" />
            Powered by WAVS
          </div>

          <h1 className="text-4xl font-bold tracking-tight text-zinc-900 sm:text-6xl dark:text-zinc-50">
            AI-Managed Trading Vault
          </h1>
        </div>
      </div>
    </div>
  );
}
