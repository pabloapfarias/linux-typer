# Pendências — linux-voice-typer

Documento vivo de bugs, correções e evoluções pendentes, organizado por
prioridade. Base: análise do código em **2026-10-12** (commit `54c9734`).

## Regras do projeto

- **O código é a fonte da verdade.** Documentação (README, TECHNICAL,
  config.example) deve ser atualizada no mesmo commit da mudança de código.
- Ao concluir um item, mova-o para a seção **Concluídos** com data e commit.
- IDs são estáveis e podem ser citados em commits: `fix: Fechar janela sem tray (P0-2)`.

## Legenda

| Nível | Significado |
|---|---|
| **P0** | Bug crítico: quebra o uso diário ou deixa o app em estado inutilizável |
| **P1** | Correção importante: comportamento errado ou perda de dados/config |
| **P2** | Evolução de notificações (OSD) |
| **P3** | Qualidade, higiene e infraestrutura |
| **P4** | Backlog de features |

## Resumo

| Prioridade | Foco | Itens |
|---|---|---|
| P0 | Bugs críticos de uso diário | 0 |
| P1 | Correções importantes | 0 |
| P2 | Notificações (evolução) | 0 |
| P3 | Qualidade e infraestrutura | 0 |
| P4 | Backlog de features | 7 |

---

## P0 — Bugs críticos

Nenhum item aberto. Os três itens originais (P0-1, P0-2 e P0-3) foram
concluídos em 2026-10-12 — ver §Concluídos.

---

## P1 — Correções importantes

Nenhum item aberto. Os itens originais (P1-1, P1-2, P1-3, P1-4 e P1-5) foram
concluídos em 2026-10-12 — ver §Concluídos.

---

## P2 — Notificações (evolução)

Nenhum item aberto. O P2-5 (flags `-e`/`-t 0` e substituição de OSD) foi
concluído em 2026-10-12 com validação no GNOME — ver §Concluídos.

---

## P3 — Qualidade e infraestrutura

Nenhum item aberto. Os itens P3-1 a P3-10 foram concluídos em 2026-10-12 —
ver §Concluídos.

---

## P4 — Backlog de features

1. Histórico de transcrições (persistência + aba na UI).
2. Escolha de microfone (listar devices via `cpal`; campo `input_device` no config).
3. Seletores nativos de arquivo para modelo e `whisper-cli` (`select_model_file()`/`select_whisper_bin()` hoje retornam `None`).
4. Suporte X11 para clipboard (`xclip`/`xsel`), se houver demanda — hoje o app é Wayland-first por causa do `wl-clipboard`.
5. Validar empacotamento `.deb` / `AppImage` (`tauri.conf.json` já define os targets, mas o build não foi testado).
6. Serviço `systemd --user`.
7. Suporte a `libei` / portal desktop (injeção mais portável que `uinput`/`ydotool`).

---

## Concluídos

| Data | Item | Commit |
|---|---|---|
| 2026-10-12 | Conciliação de README, TECHNICAL e config.example com o comportamento do código | `54c9734` |
| 2026-10-12 | P0-1 — OSD preso em erro/parada (notificação de limpeza em `Idle`) | `cb68d7d` |
| 2026-10-12 | P0-2 — fechar janela sem tray (`close_to_tray` + detecção de host StatusNotifier) | `cb68d7d` |
| 2026-10-12 | P0-3 — aviso de hotkey global indisponível (evento + OSD) | `cb68d7d` |
| 2026-10-12 | P1-4 — memory leak do preview no OSD de conclusão (corrigido junto com P0-1) | `cb68d7d` |
| 2026-10-12 | P1-1 — corrida de estado entre transcrição e nova gravação (sessão/geração) | `7683a43` |
| 2026-10-12 | P1-3 — limpeza de WAV/TXT temporários após cada ditada | `7683a43` |
| 2026-10-12 | P2-1 — OSD de erro com mensagem amigável | `7683a43` |
| 2026-10-12 | P2-2 — OSD de ciclo de vida + campo `osd_enabled` (parte do P2-3) | `7683a43` |
| 2026-10-12 | P1-2 — `start_minimized` implementado (janela oculta com tray + fallback seguro) | `6d14d0f` |
| 2026-10-12 | P1-5 — salvar pela UI preserva `auto_paste`/`trim_text`/`insert_mode` | `6d14d0f` |
| 2026-10-12 | P2-3 — disponibilidade do `notify-send` verificada (ícones seguem o spec freedesktop) | `6d14d0f` |
| 2026-10-12 | P2-4 — OSD extraído para `src/notify.rs` | `6d14d0f` |
| 2026-10-12 | P2-5 — flags `-e`/`-t 0` validadas no GNOME; substituição de OSD corrigida (ID real do daemon) | `426fb8c` |
| 2026-10-12 | P3-1 — clippy zerado (core e desktop compilam com `-D warnings`) | `665019b` |
| 2026-10-12 | P3-2 — código morto removido | `665019b` |
| 2026-10-12 | P3-3 — tray "Iniciar" reporta erro de validação via OSD | `665019b` |
| 2026-10-12 | P3-4 — `save_config` emite aviso quando a config salva não valida | `665019b` |
| 2026-10-12 | P3-5 — CLI `run` retorna erro com ambiente incompleto | `665019b` |
| 2026-10-12 | P3-6 — testes de config + workflow de CI (fmt/test/clippy) | `665019b` |
| 2026-10-12 | P3-7 — script `execute` versionado e documentado no README | `44849cd` |
| 2026-10-12 | P3-8 — placeholders `bin/` e `models/` removidos | `665019b` |
| 2026-10-12 | P3-9 — detecção de microfone com cache de 10s | `665019b` |
| 2026-10-12 | P3-10 — botões Iniciar/Pausar com ações dedicadas | `665019b` |
