# Установка

## Готовый бинарь

```sh
curl -fsSL https://raw.githubusercontent.com/maximusmakarov/deepseek_harness_rust/main/scripts/install.sh | bash
# или из клона:
scripts/install.sh
```

Установщик скачивает `dshr-<version>-<target>.tar.gz`, сверяет `sha256` с
`SHA256SUMS` и кладёт бинарь в `~/.local/bin` (`DSHR_INSTALL_DIR`). Версию, таргет и
базовый URL можно задать через `DSHR_VERSION`, `DSHR_TARGET`,
`DSHR_RELEASE_BASE_URL`. Поддерживаются macOS (arm64/x86_64) и Linux (x86_64/aarch64).

Проверка вручную:

```sh
shasum -a 256 -c SHA256SUMS
dshr release verify SHA256SUMS --signature SHA256SUMS.sig --key "$(cat SHA256SUMS.pub)"
gh attestation verify dshr-<version>-<target>.tar.gz --repo maximusmakarov/deepseek_harness_rust
```

## Homebrew

Стабильная бинарная формула публикуется в каждом релизе как `dshr.rb`:

```sh
curl -fsSLO https://github.com/maximusmakarov/deepseek_harness_rust/releases/latest/download/dshr.rb
brew install ./dshr.rb
```

Формула ставит готовый бинарь из архива с закреплённым `sha256`; ghcr-бутылок нет.
HEAD-вариант собирает из `main`:

```sh
brew install --HEAD ./packaging/homebrew/Formula/dshr.rb
```

## Cargo

```sh
cargo install --git https://github.com/maximusmakarov/deepseek_harness_rust \
  --locked --package dshr-cli
```

или `scripts/install.sh --from-source`.

Полный процесс релиза — [RELEASE.md](RELEASE.md).

## Сервис

- systemd: `packaging/systemd/dshr.service` (подставьте пользователя и токен).
- launchd: `packaging/launchd/com.dshr.server.plist`.

Токен: `DSHR_API_TOKEN`; без него сервер генерирует токен при старте.

## Docker

```sh
docker build -t dshr .
docker run --rm -p 127.0.0.1:4317:55678 -e DSHR_API_TOKEN=... dshr
```

## PWA

`dshr serve`, открыть `http://127.0.0.1:4317`, установить приложение из браузера.
