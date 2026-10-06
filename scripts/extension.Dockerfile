# Reproducible extension build, for store reviewers and anyone verifying a release:
#   docker build -f scripts/extension.Dockerfile -o out .
# out/ then holds the Firefox build; compare it with the files inside the published zip.
FROM node:24-bookworm AS build
RUN corepack disable 2>/dev/null; npm install -g pnpm@11.15.0
# rust-toolchain.toml pins the exact Rust version; rustup installs it on first use.
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
ENV PATH=/root/.cargo/bin:$PATH
WORKDIR /src
COPY . .
RUN sh scripts/install-wasm-bindgen.sh \
 && pnpm install --frozen-lockfile \
 && pnpm run build:wasm \
 && pnpm --filter @scytale/extension run build:firefox \
 && pnpm --filter @scytale/extension run build

FROM scratch
COPY --from=build /src/apps/extension/.output/firefox-mv3 /firefox-mv3
COPY --from=build /src/apps/extension/.output/chrome-mv3 /chrome-mv3
