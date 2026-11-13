# 027. Linux seccomp: сетевая изоляция

- **Статус:** Accepted
- **Дата:** 2026-11-13

## Контекст

Landlock ABI V1 ограничивает только файловые эффекты; `network: deny` на Linux фактически не действовал. Для shell/jobs нужен барьер на сетевые syscalls, не ломающий файловую песочницу.

## Решение

1. **seccomp-bpf через `seccompiler`.** В Linux-helper после Landlock ставится фильтр: перечисленные сетевые syscalls возвращают `EPERM`, все остальные разрешены.
2. **Список:** `socket`, `socketpair`, `connect`, `bind`, `listen`, `accept`, `accept4`, `sendto`, `recvfrom`, `sendmsg`, `recvmsg`, `sendmmsg`, `recvmmsg`, `getsockname`, `getpeername`, `setsockopt`, `getsockopt`, `shutdown`.
3. **Порядок:** Landlock → seccomp → `exec`. `PR_SET_NO_NEW_PRIVS` выставляет `seccompiler`.
4. **Enforcement:** Linux теперь `Full` (FS + сеть при `deny`); Windows остаётся `Partial` (AppContainer без гарантии сети).
5. **Границы:** блокируются и Unix-сокеты (`socketpair`); сетевые гарантии не распространяются на Windows.

## Последствия

- `network: deny` реально запрещает сеть в Linux shell; тест запускает `socket()` и проверяет `EPERM`.
- Некоторые программы, использующие Unix-сокеты, могут сломаться — это осознанная строгость.
- seccomp не является полным syscall-фильтром; он закрывает сетевой вектор.

## Альтернативы

- Landlock ABI V4 network — отложено: покрывает только TCP bind/connect, не UDP/DNS.
- Полный allowlist syscalls — отложено: высокий риск сломать shell.
- `bwrap` — отклонено: внешняя зависимость.
