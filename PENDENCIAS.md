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
| P1 | Correções importantes | 2 |
| P2 | Notificações (evolução) | 3 |
| P3 | Qualidade e infraestrutura | 10 |
| P4 | Backlog de features | 7 |

---

## P0 — Bugs críticos

Nenhum item aberto. Os três itens originais (P0-1, P0-2 e P0-3) foram
concluídos em 2026-10-12 — ver §Concluídos.

---

## P1 — Correções importantes

### P1-2 · `start_minimized` é salvo, mas não tem efeito
- **Tipo:** feature incompleta · **Área:** desktop / config
- **Onde:** `src/config.rs:78`; `apps/desktop/src-tauri/src/lib.rs:230-237` e `:410`,`:433`; `apps/desktop/src/main.js:854`,`:873`; `src/autostart.rs`
- **Sintoma:** o checkbox "Iniciar Minimizado na Bandeja" salva o campo, mas nenhum código o consome; a janela sempre abre visível (inclusive no autostart).
- **Decisão necessária:** implementar (janela `visible: false` quando `start_minimized`; flag no `.desktop` do autostart) **ou** remover o checkbox/campo e documentar como não suportado.
- **Doc afetado:** `README.md` §Ações na Interface, `TECHNICAL.md` §Limitações, `config.example.toml`.

### P1-5 · Salvar pela UI sobrescreve configurações existentes
- **Tipo:** bug · **Área:** frontend
- **Onde:** `apps/desktop/src/main.js:858-876` (`collectConfig()`)
- **Sintoma:** o salvamento força `auto_paste: true`, `trim_text: true` e `insert_mode: "clipboard"`, descartando valores custom do `config.toml` editados manualmente.
- **Correção sugerida:** preservar os valores de `currentConfig` (remover os overrides fixos) ou expor controles de `auto_paste`/`trim_text` na aba Configurações.

---

## P2 — Notificações (evolução)

### P2-3 · Robustez do `notify-send`
- **Onde:** `src/service.rs` (`send_osd`)
- **Itens:** verificar disponibilidade do `notify-send` (hoje o erro é ignorado silenciosamente); fallback de ícones independente do tema. O campo `osd_enabled` foi implementado junto com P2-2.

### P2-4 · Extrair OSD para módulo dedicado
- **Onde:** hoje tudo em `src/service.rs:429-482`
- **Objetivo:** criar `src/notify.rs` com API testável (título, corpo, estado, expiração), separando notificação do loop de serviço.

### P2-5 · Revisar `-e` (transient) + `-t 0` por daemon
- **Objetivo:** validar comportamento no GNOME e no KDE/Plasma; documentar as diferenças observadas e ajustar flags se necessário.

---

## P3 — Qualidade e infraestrutura

### P3-1 · Zerar warnings do clippy
- **Onde:** `src/audio.rs:173` (clone em `Copy`), `src/audio.rs:193` (closure redundante), `src/config.rs:172` (`cmp_owned`), `src/service.rs:284` (`if` colapsável) e `apps/desktop/src-tauri/src/lib.rs:366` (`if` colapsável no handler do tray)

### P3-2 · Remover código morto
- **Onde:** `clipboard::_as_os_str` (`src/clipboard.rs:121-124`), `setup::_canonical` (`src/setup.rs:351-354`)

### P3-3 · Feedback ao iniciar pelo tray
- **Onde:** `apps/desktop/src-tauri/src/lib.rs:323-329` — erro de config inválida é ignorado silenciosamente no menu "Iniciar"; publicar evento/toast ou OSD.

### P3-4 · Validação no `save_config`
- **Onde:** `apps/desktop/src-tauri/src/lib.rs:90-116` — hoje grava sem validar caminhos; o erro só aparece na primeira ditada. Avaliar validar antes de gravar ou avisar sem bloquear.

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
- **Onde:** `VoiceTyperService::status()` → `default_microphone_name()` (`src/service.rs:508-514`), chamado pela UI a cada 1s (`apps/desktop/src/main.js:939`). Cachear o nome/estado do microfone.

### P3-10 · Revisar botões Iniciar/Pausar duplicados
- **Onde:** `apps/desktop/src/main.js:956-976` — "Iniciar" e "Pausar" usam o mesmo handler de alternância; revisar rótulos/estado para evitar ação invertida.

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
