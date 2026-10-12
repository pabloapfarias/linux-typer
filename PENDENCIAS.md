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
| P2 | Notificações (evolução) | 1 |
| P3 | Qualidade e infraestrutura | 10 |
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

### P2-5 · Revisar `-e` (transient) + `-t 0` por daemon
- **Status parcial (GNOME):** validado com `notify-send 0.8.8` — `-e`/`-t 0` funcionam, mas o ID fixo **não** substituía a notificação anterior; corrigido em `src/notify.rs` (guarda o ID do daemon via `-p` e usa `-r <id>`).
- **Falta:** validar no KDE/Plasma e documentar diferenças, se houver.

---

## P3 — Qualidade e infraestrutura

### P3-1 · Zerar warnings do clippy
- **Onde:** `src/audio.rs:173` (clone em `Copy`), `src/audio.rs:193` (closure redundante), `src/config.rs:178` (`cmp_owned`), `src/service.rs:321` (`if` colapsável) e `apps/desktop/src-tauri/src/lib.rs:375` (`if` colapsável no handler do tray)

### P3-2 · Remover código morto
- **Onde:** `clipboard::_as_os_str` (`src/clipboard.rs:121-124`), `setup::_canonical` (`src/setup.rs:351-354`)

### P3-3 · Feedback ao iniciar pelo tray
- **Onde:** `apps/desktop/src-tauri/src/lib.rs:342-354` — erro de config inválida é ignorado silenciosamente no menu "Iniciar"; publicar evento/toast ou OSD.

### P3-4 · Validação no `save_config`
- **Onde:** `apps/desktop/src-tauri/src/lib.rs:98-124` — hoje grava sem validar caminhos; o erro só aparece na primeira ditada. Avaliar validar antes de gravar ou avisar sem bloquear.

### P3-5 · Exit code do CLI em ambiente incompleto
- **Onde:** `src/main.rs:44-53` — `run` retorna sucesso (exit 0) mesmo com ambiente incompleto; usar exit code de erro para automação.

### P3-6 · Testes e CI
- **Estado atual:** 2 testes (`hotkey`, `service`) com `sleep` (flaky potencial); sem CI.
- **Objetivo:** testes unitários para `config`, `transcriber::cleanup_transcript`, mapeamento do `injector` e OSD/estado; workflow GitHub Actions com `cargo test`, `clippy` e `fmt`.

### P3-7 · Decidir destino do script `execute`
- **Situação:** está na raiz, fora do git (`?? execute`). Versionar e documentar no README/TECHNICAL **ou** remover do diretório de trabalho.

### P3-8 · Placeholders `bin/` e `models/`
- **Situação:** contêm apenas `.gitkeep` e não são usados pelo código (tudo vai para `~/.local/share/linux-voice-typer/`). Remover ou explicar a finalidade.

### P3-9 · Polling de status chama `cpal` a cada segundo
- **Onde:** `VoiceTyperService::status()` → `default_microphone_name()` (`src/service.rs:631-637`), chamado pela UI a cada 1s (`apps/desktop/src/main.js:960`). Cachear o nome/estado do microfone.

### P3-10 · Revisar botões Iniciar/Pausar duplicados
- **Onde:** `apps/desktop/src/main.js:977-997` — "Iniciar" e "Pausar" usam o mesmo handler de alternância; revisar rótulos/estado para evitar ação invertida.

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
