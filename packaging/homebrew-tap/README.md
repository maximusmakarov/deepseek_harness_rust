# Tap для `dshr`

Этот каталог — шаблон содержимого отдельного репозитория **tap**, без которого
`brew install maximusmakarov/dshr/dshr` невозможен. Сам tap живёт вне основного
репозитория (`maximusmakarov/homebrew-dshr`), потому что Homebrew определяет tap
по имени репозитория `homebrew-<tap>`.

## Что здесь лежит

Формулу не хранят вручную: её рендерит релизный прогон
(`.github/workflows/release.yml`, шаг `render ghcr formula`) в
`dist/dshr-ghcr.rb`, а в tap её кладёт `scripts/sync-homebrew-tap.sh`.

```
packaging/homebrew-tap/
└── README.md          # этот файл: раскладка tap и порядок действий
```

## Одноразовая настройка (владелец)

Сделано: репозиторий `maximusmakarov/homebrew-dshr` создан, формула `0.1.0`
опубликована. Ниже — что нужно повторить при пересоздании tap.

1. Создать публичный репозиторий `maximusmakarov/homebrew-dshr`.
2. Склонировать его рядом с основным репозиторием:

   ```sh
   git clone git@github.com:maximusmakarov/homebrew-dshr.git ../homebrew-dshr
   ```

3. Убедиться, что пакет `ghcr.io/maximusmakarov/deepseek_harness_rust/dshr`
   публичный: brew тянет блоб анонимно, приватный пакет даёт 404 на `brew install`.

## Выпуск версии

```sh
# после релизного прогона на теге v0.1.0 (dist/dshr-ghcr.rb собран из артефактов)
scripts/sync-homebrew-tap.sh ../homebrew-dshr --formula dist/dshr-ghcr.rb --dry-run
scripts/sync-homebrew-tap.sh ../homebrew-dshr --formula dist/dshr-ghcr.rb --push
```

Скрипт проверяет `root_url "https://ghcr.io/v2/..."`, наличие bottle-`sha256`,
`ruby -c`, копирует формулу в `Formula/dshr.rb`, коммитит `dshr <version>` и с
`--push` отправляет в tap. Версия читается из URL релиза (строки `version` в
формуле нет — её отбраковывает `brew audit --strict`). Повторный запуск с той же
формулой не создаёт пустой коммит.

Сгенерированная формула проходит `brew audit --strict` без замечаний
(проверено на Homebrew 7.0.13): блок `bottle do` отсортирован
(ARM macOS → Intel macOS → ARM Linux → Intel Linux) и digests выровнены.

## Проверка для пользователя

```sh
brew tap maximusmakarov/dshr
brew trust maximusmakarov/dshr          # Homebrew 7: иначе «untrusted tap»
brew install maximusmakarov/dshr/dshr
brew test dshr
dshr --version
```

`brew install` должен именно **набирать** бутылку (`pouring`), а не собирать из
исходников: иначе `bottle do` не совпал с платформой или индекс не содержит
нужный манифест (`brew install -v` показывает выбор манифеста).

## Проверено на 0.1.0 (16.11.2026)

`brew tap maximusmakarov/dshr` видит формулу, `brew audit --strict` — 0
замечаний, `brew fetch maximusmakarov/dshr/dshr` скачал манифест индекса и
бутылку `arm64_sequoia` из GHCR анонимно, sha256 совпал с формулой и с ассетом
релиза `v0.1.0` (`1f086b6a…`), `brew install maximusmakarov/dshr/dshr` налил
бутылку (`poured_from_bottle: true`), `brew test dshr` и `dshr --version`
проходят. Индекс содержал все четыре платформы.

## Ограничения

- Если в другом tap есть формула с тем же именем, brew отказывается резолвить
  короткое имя (`Formulae found in multiple taps`). Лечится полным именем
  `maximusmakarov/dshr/dshr` или `brew untap <лишний-tap>`.

- Индекс `ghcr.io/<owner>/<repo>/dshr:<version>` один на версию и содержит все
  собранные платформы (`sonoma`, `arm64_sequoia`, `x86_64_linux`,
  `aarch64_linux`); отсутствующая платформа — не ошибка рендера, а отсутствие
  соответствующей бутылки в релизном артефакте.
- Формула не подписана отдельно от бандла-архива: доверие к бутылке — это
  доверие к GHCR-пакету и его владельцу.
- Notarization к бутылкам не применяется (только к macOS `.app`).
