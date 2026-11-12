# LSP

LSP-серверы подключаются через конфиг и дают агенту навигацию и диагностику.

```yaml
lsp:
  servers:
    - name: rust
      command: rust-analyzer
      extensions: ["rs"]
    - name: typescript
      command: typescript-language-server
      args: ["--stdio"]
      extensions: ["ts", "tsx"]
```

Инструменты:

| Инструмент | Аргументы | Что делает |
|---|---|---|
| `lsp_hover` | `path`, `line`, `column` | hover в позиции (1-based) |
| `lsp_definition` | `path`, `line`, `column` | переход к определению |
| `lsp_references` | `path`, `line`, `column` | ссылки с `includeDeclaration` |
| `lsp_diagnostics` | `path` | диагностика после открытия файла |

Особенности:

- Сервер запускается лениво при первом обращении к файлу с подходящим расширением.
- Файл открывается через `textDocument/didOpen`; изменения на диске подхватываются повторным открытием.
- Диагностика приходит асинхронно (`publishDiagnostics`); инструмент ждёт 300 мс.
- `initialize` объявляет пустые capabilities: completion, rename, code actions пока не используются.
- Один сервер выбирается по расширению; несколько серверов на одно расширение не поддерживаются.

Пример конфига для проверки без реального сервера — `mock_lsp` в тестах.
