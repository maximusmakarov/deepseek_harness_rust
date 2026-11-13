# Установка

## Cargo

```sh
cargo install --git https://github.com/maximusmakarov/deepseek_harness_rust \
  --locked --package dshr-cli
```

или `scripts/install.sh`.

## Homebrew (HEAD)

```sh
brew install --HEAD ./packaging/homebrew/Formula/dshr.rb
```

Формула собирает из `main`; стабильный tap/бинарные бутылки не настроены.

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
