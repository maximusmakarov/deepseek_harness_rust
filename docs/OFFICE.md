# Office

Инструменты для docx/xlsx/pptx.

| Инструмент | Назначение |
|---|---|
| `office_read` | текст из docx/xlsx/pptx (по расширению) |
| `office_create_docx` | docx с заголовком и абзацами |
| `office_create_xlsx` | xlsx с именованными листами и строковыми ячейками |
| `office_create_pptx` | pptx с титульным слайдом и слайдами-буллетами |

Примеры:

```json
{"path": "report.docx", "title": "Report", "paragraphs": ["First", "Second"]}
```

```json
{"path": "data.xlsx", "sheets": [{"name": "Data", "rows": [["name", "value"], ["a", "1"]]}]}
```

```json
{"path": "deck.pptx", "title": "Deck", "slides": [{"title": "Intro", "bullets": ["one", "two"]}]}
```

```json
{"path": "deck.pptx"}
```

Особенности:

- docx читается через `word/document.xml` (абзацы/таблицы), стили и изображения не извлекаются.
- xlsx читается через `calamine`; значения приводятся к тексту.
- pptx читается через `pptx`: текстовые AutoShape; таблицы/диаграммы/заметки не извлекаются.
- Создание — базовое: без стилей, изображений и формул.
- pptx собирается из встроенного MIT-шаблона (мастера/layout/theme), слайды и текст генерируются локально.
- OOXML-валидность проверяется структурой ZIP и нашими парсерами; офисный пакет в CI не запускается.
- Файлы создаются относительно workspace; инструменты требуют approval, как остальные write-операции.
