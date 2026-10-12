# linux-voice-typer — Documentacao tecnica

`linux-voice-typer` e um app Linux-first (Wayland) para ditado local com Whisper e colagem automatica no input ativo.

O projeto tem duas camadas:

- o core Rust com CLI (`cargo run -- ...`), base de debug e uso avancado;
- a interface desktop opcional em Tauri v2 (`apps/desktop/`), que consome o mesmo core.

Fluxo principal:

`segurar atalho -> gravar microfone -> soltar atalho -> transcrever localmente -> copiar para clipboard -> colar`

## Instalacao rapida

```bash
git clone <repo>
cd linux-voice-typer
cargo run -- setup
cargo run -- doctor
cargo run -- paste-test
cargo run -- run --editor-paste
```

Para usar dentro de terminal, como Codex CLI:

```bash
cargo run -- run --terminal-hotkey --terminal-paste
```

## Comandos

```bash
cargo run -- setup
cargo run -- doctor
cargo run -- paste-test
cargo run -- run
cargo run -- run --editor-paste
cargo run -- run --terminal-paste
cargo run -- run --terminal-hotkey --terminal-paste
```

Flags do `setup`:

```bash
cargo run -- setup --yes                  # nao pede confirmacao
cargo run -- setup --model medium         # tiny, base, small (default) ou medium
cargo run -- setup --skip-system-deps     # nao checa dependencias do sistema
cargo run -- setup --skip-whisper-build   # nao compila o whisper.cpp
cargo run -- setup --skip-model-download  # nao baixa o modelo
cargo run -- setup --rebuild-whisper      # recompila mesmo se o binario existir
cargo run -- setup --force                # baixa/atualiza mesmo se ja existir
```

Flag global (aceita em qualquer subcomando, mas so faz sentido no `run`):

```bash
# --terminal-hotkey: Enter inicia e Enter para parar a gravacao (sem hotkey global)
cargo run -- run --terminal-hotkey
```

Com binario buildado:

```bash
cargo build --release
./target/release/linux-voice-typer setup
./target/release/linux-voice-typer doctor
./target/release/linux-voice-typer paste-test
./target/release/linux-voice-typer run
./target/release/linux-voice-typer run --editor-paste
./target/release/linux-voice-typer run --terminal-paste
./target/release/linux-voice-typer run --terminal-hotkey --terminal-paste
```

`--editor-paste` e `--terminal-paste` sao mutuamente exclusivos e so
sobrescrevem o atalho durante aquela execucao.

Servico `systemd --user`:

```bash
cargo run -- service install             # instala a unit, habilita e inicia
cargo run -- service install --no-enable # apenas instala a unit
cargo run -- service status
cargo run -- service uninstall
```

## Uso diario

O programa sempre transcreve para o clipboard primeiro. Depois ele envia um
atalho de colagem para a janela ativa.

Use o modo conforme o alvo:

```bash
# Editores, navegadores e inputs comuns: envia Ctrl+V
cargo run -- run --editor-paste

# Terminais Linux, incluindo Codex CLI: envia Ctrl+Shift+V
cargo run -- run --terminal-paste

# Modo de hotkey por terminal: Enter inicia e Enter para parar a gravacao
cargo run -- run --terminal-hotkey --terminal-paste
```

Se nenhum override for informado, o app usa `paste_shortcut` do `config.toml`.
Neste projeto, a configuracao local recomendada para editor comum e:

```toml
paste_backend = "uinput"
paste_shortcut = "ctrl_v"
restore_clipboard = false
```

## Auto-paste no Wayland

O auto-paste no Wayland depende do compositor e do backend configurado.

- `wtype` nao funciona em todos os compositores Wayland.
- Se aparecer `Compositor does not support the virtual keyboard protocol`, isso nao e erro do Whisper.
- Nesse caso, use `paste_backend = "uinput"` ou `paste_backend = "ydotool"`.
- Mesmo sem auto-paste, a transcricao continua no clipboard.

Backend nativo recomendado sem `wtype`:

```toml
paste_backend = "uinput"
paste_shortcut = "ctrl_v"
restore_clipboard = false
```

O backend `uinput` cria um teclado virtual temporario em `/dev/uinput` e envia
o atalho de colagem. Ele nao depende do protocolo `virtual-keyboard` do
compositor Wayland.

No GNOME/Wayland, esse e o caminho mais confiavel quando `wtype` falha.
O `doctor` valida se `/dev/uinput` esta acessivel e se o teclado virtual pode
ser criado.

Comandos uteis para `ydotool`:

```bash
sudo apt install -y ydotool
sudo modprobe uinput
sudo ydotoold
```

Teste manual com clipboard:

```bash
echo "teste do linux voice typer" | wl-copy
ydotool key 29:1 47:1 47:0 29:0
```

Para muitos terminais Linux:

```bash
ydotool key 29:1 42:1 47:1 47:0 42:0 29:0
```

## A transcricao aparece no log, mas nao cola

Se o log mostra `transcript copied to clipboard` mas nada aparece no alvo:

1. Verifique se o clipboard realmente tem o texto:

```bash
wl-paste
```

2. Se `wl-paste` nao mostrar a transcricao, o problema e no clipboard (nao no Whisper).

3. Teste o clipboard isoladamente:

```bash
cargo run -- paste-test
```

Depois do teste, rode `wl-paste` para verificar se o texto permanece.

4. Confirme se o atalho combina com o alvo:

```bash
# input/editor comum
cargo run -- run --editor-paste

# terminal
cargo run -- run --terminal-paste
```

5. Se o texto some do clipboard rapidamente, verifique a config:

```toml
paste_backend = "none"
restore_clipboard = false
```

Com `restore_clipboard = false`, a transcricao fica no clipboard indefinidamente.

6. Se `restore_clipboard = true`, o clipboard so e restaurado quando o auto-paste
   confirma sucesso. Se o auto-paste falhar, a transcricao permanece.

## Configuracao de paste

Configuracao recomendada para auto-paste no Wayland sem `wtype`:

```toml
paste_backend = "uinput"
paste_shortcut = "ctrl_v"
restore_clipboard = false
```

Exemplo para apps normais:

```toml
paste_backend = "uinput"
paste_shortcut = "ctrl_v"
restore_clipboard = false
```

Exemplo para terminais:

```toml
paste_backend = "uinput"
paste_shortcut = "ctrl_shift_v"
restore_clipboard = false
```

Tambem e possivel manter `paste_shortcut = "ctrl_v"` no config e escolher por
execucao:

```bash
cargo run -- run --editor-paste
cargo run -- run --terminal-paste
```

Esses flags nao alteram o arquivo `config.toml`; eles so sobrescrevem o atalho
durante aquela execucao.

Modo mais seguro:

```toml
paste_backend = "none"
restore_clipboard = false
```

## O que o setup faz

`setup` tenta reduzir o atrito inicial:

1. detecta Linux, Wayland e package manager
2. detecta dependencias ausentes
3. oferece instalacao automatica no Ubuntu/Debian via `apt`
4. clona `whisper.cpp` em `~/.local/share/linux-voice-typer/whisper.cpp`
5. compila `whisper.cpp`
6. baixa um modelo Whisper (default `small`; `--model` escolhe tiny/base/small/medium)
7. cria ou atualiza `config.toml` com caminhos reais
8. cria `config.toml.bak` antes de sobrescrever um config existente

## O que o doctor verifica

- Linux
- Wayland
- ferramenta de clipboard: `wl-copy`/`wl-paste` (Wayland) ou `xclip`/`xsel` (X11)
- teste real de clipboard (escreve e le de volta)
- `wtype`
- `ydotool`
- `ydotoold`
- `/dev/uinput`
- criacao real de teclado virtual via `uinput`
- `clipboard fallback`
- `config.toml`
- `whisper-cli`
- `model`
- `temp_dir`
- microfone padrao via `cpal`
- inicializacao do `keytap`

Os itens reportados como `ERROR` (Linux, ferramenta de clipboard, `config.toml`,
`whisper-cli`, `model`, `temp_dir` gravavel e microfone) bloqueiam o comando
`run`. Quando o ambiente esta incompleto e o terminal e interativo, o `run`
oferece executar o `setup` na hora.

## Interface desktop / Tauri

A interface desktop fica em:

```text
apps/desktop/
```

Ela usa:

- Tauri v2
- Rust no backend
- HTML/CSS/JS simples no frontend
- Vite apenas para desenvolvimento/build do frontend

O core continua no crate Rust principal. A interface desktop depende do crate
`linux-voice-typer` via path e chama os mesmos modulos de configuracao,
doctor, paste-test, audio, transcricao, clipboard e injector.

### Instalar dependencias da interface

```bash
cd apps/desktop
npm install
```

Dependencias Linux do Tauri/WebKit podem variar por distribuicao. Em Ubuntu,
se o build falhar por WebKit/AppIndicator, instale as dependencias indicadas
pelo erro do Tauri/WebKitGTK.

### Rodar em modo desenvolvimento

```bash
cd apps/desktop
npm run tauri dev
```

Esse comando inicia o Vite e abre a janela Tauri.

### Build sem empacotar

```bash
cd apps/desktop
npm run tauri -- build --debug --no-bundle
```

O binario debug gerado fica em:

```text
apps/desktop/src-tauri/target/debug/linux-voice-typer-desktop
```

### Build com empacotamento

```bash
cd apps/desktop
npm run tauri -- build --bundles deb       # pacote .deb
npm run tauri -- build --bundles appimage  # AppImage
```

Os artefatos ficam em:

```text
apps/desktop/src-tauri/target/release/bundle/deb/
apps/desktop/src-tauri/target/release/bundle/appimage/
```

O `.deb` foi validado em Ubuntu (contem binario, icones, `.desktop` e
dependencias `libwebkit2gtk-4.1-0`, `libgtk-3-0` e `libayatana-appindicator3-1`)
e o AppImage foi gerado com sucesso (~86 MiB).

### Comandos Tauri expostos

O frontend nao executa shell diretamente. Ele chama comandos Rust via Tauri:

```text
get_status()
start_service()
stop_service()
restart_service()
get_config()
save_config(config)
run_doctor()
run_paste_test()
open_config_file()
select_model_file()
select_whisper_bin()
list_input_devices()
get_history()
clear_history()
get_recent_logs()
set_mode_editor()
set_mode_terminal()
```

`select_model_file()` e `select_whisper_bin()` usam o seletor nativo de
arquivos (`tauri-plugin-dialog`) e devolvem o caminho escolhido (ou `null` se o
usuario cancelar). `list_input_devices()` lista os microfones via `cpal`.

### Servico interno

A interface usa `VoiceTyperService`, exposto pelo core em `src/service.rs`.

API principal:

```text
start()
stop()
restart()
is_running()
reload_config()
set_paste_shortcut()
set_paste_backend()
set_state_listener()
last_transcript()
status()
recent_events()
```

`set_state_listener()` recebe um callback (`DictationFeedback` com estado,
nivel de audio, tempo decorrido e transcricao) chamado a cada mudanca de
estado. A interface desktop usa esse callback para atualizar o tray icon e o
HUD em tempo real.

O serviço cria um loop em thread separada, escuta a hotkey global e usa o mesmo
pipeline validado pela CLI:

```text
hotkey start
  -> Recorder::start()
hotkey stop
  -> stop_and_save()
  -> transcriber::transcribe()
  -> injector::inject_text()
```

### Iniciar e parar pela interface

- `Iniciar` carrega `config.toml`, valida caminhos e inicia o serviço.
- `Parar` sinaliza encerramento do loop e aguarda a thread finalizar.
- `Reiniciar Serviço` (aba Diagnóstico & Debug) reinicia o loop com a config atual.

`save_config()` grava o `config.toml` e, se o serviço estava rodando, reinicia
o serviço automaticamente para que a thread use os novos valores.

### Modo Editor e modo Terminal

A UI possui dois modos:

```text
Editor/Input comum -> paste_shortcut = "ctrl_v"
Terminal           -> paste_shortcut = "ctrl_shift_v"
```

Os botões de modo chamam:

```text
set_mode_editor()
set_mode_terminal()
```

Esses comandos atualizam o `config.toml` e recarregam o estado do serviço.

### Backend de paste

A UI permite escolher:

```text
uinput
ydotool
wtype
libei
none
auto
```

Recomendado no GNOME/Wayland:

```toml
paste_backend = "uinput"
restore_clipboard = false
```

### Backend libei (portal XDG)

O backend `libei` usa o protocolo EI via portal `RemoteDesktop`: na primeira
ditada o desktop mostra um dialogo de consentimento e a sessao fica
reutilizada pelo processo (o dialogo nao aparece de novo). Diferente de
uinput/ydotool, nao exige `/dev/uinput` nem o grupo `input`, funcionando tambem
em ambiente sandbox (Flatpak).

- nao faz parte do modo `auto`: o dialogo so aparece se o backend for escolhido;
- validado no GNOME 50 + `xdg-desktop-portal-gnome` (`ConnectToEIS` v2): handshake, evento `resumed` e envio de Ctrl+V confirmados;
- o dialogo de consentimento do GNOME aparece como **"Area de trabalho remota / Permitir interacao remota"** e nao pode ser personalizado pelo app; isso pode assustar usuarios de ditado, por isso o backend e experimental e nao e o padrao;
- a sessao e por processo: encerrar o app exige novo consentimento;
- requer um portal XDG ativo (o `doctor` reporta `XDG portal available`).

### Tray icon

O app cria um tray icon com menu:

```text
Linux Voice Typer
Iniciar
Pausar
Modo Editor
Modo Terminal
Abrir Configurações
Rodar Doctor
Sair
```

Comportamento:

- fechar a janela esconde o app quando existe host de tray e `close_to_tray = true` (default)
- a deteccao do host usa o watcher `org.kde.StatusNotifierWatcher` via D-Bus (`gdbus`)
- sem host StatusNotifier detectado (ex.: GNOME sem extensao AppIndicator), fechar a janela encerra o app
- com `close_to_tray = false`, fechar a janela tambem encerra o app
- com `start_minimized = true` e host de tray presente, o app inicia com a janela oculta; sem tray, a janela abre mesmo assim
- o serviço pode continuar rodando pelo tray
- `Abrir Configurações` mostra a janela novamente
- `Sair` encerra o app de verdade

Estados do icone:

- verde: serviço rodando (idle)
- vermelho: gravando
- cinza: serviço parado
- durante a transcricao o icone permanece verde

No GNOME, a exibição do tray/AppIndicator pode depender de suporte do ambiente
ou de extensão compatível com AppIndicator/KStatusNotifier.

### Logs e eventos visuais

O serviço mantém os últimos eventos em memória. Exemplos:

```text
14:06 Serviço iniciado
14:06 Hotkey global ativa: F7
14:16 Gravação iniciada
14:16 Gravação finalizada
14:16 Transcrição concluída
14:16 Texto colado com Ctrl+V
```

Se o hotkey global não puder ser registrado, o evento correspondente é
"Hotkey global indisponível; modo terminal ativo (sem suporte na GUI)" e um
OSD de aviso é exibido.

A tela mostra eventos resumidos. Logs técnicos detalhados continuam mais úteis
pelo console e pela CLI.

### Notificações OSD

Quando o serviço (interface desktop) muda de estado, o core dispara uma
notificação de desktop via `notify-send`. O ID atribuído pelo daemon é guardado
(`-p`) e o aviso seguinte usa `-r <id>`, substituindo o anterior:

| Estado | Icone | Expira | Texto |
|---|---|---|---|
| Gravando | `audio-input-microphone` | nunca (`-t 0`) | "🎙️ Escutando..." / "Fale agora no microfone..." |
| Transcrevendo | `system-run` | nunca (`-t 0`) | "⏳ Transcrevendo..." / "Processando áudio com Whisper..." |
| Concluído | `emblem-default` | 3,5s | "✓ Ditado Colado!" + preview de ate 60 caracteres |
| Ocioso (limpeza) | `edit-clear` | 1,5s | "🎙️ Pronto" / "Pode ditar novamente quando quiser." |
| Aviso | `dialog-warning` | 5s | "⚠️ Hotkey global indisponível" / detalhe do problema |
| Erro | `dialog-error` | 4s | "⚠️ Falha ao gravar" / "⚠️ Ditado falhou" + mensagem amigável |
| Ciclo de vida | `media-playback-start` / `media-playback-pause` | 2,5s | "▶️ Serviço iniciado" / "⏸️ Serviço parado" |

Detalhes e limitações atuais:

- a CLI pura não envia notificações; o OSD é exclusivo do serviço da UI desktop.
- o campo `osd_enabled` (default `true`) desativa todo o OSD.
- o OSD de ciclo de vida é suprimido em reinícios internos (ex.: salvar configurações).
- sem o binário `notify-send`, o OSD fica desativado (aviso único no log).
- os nomes de ícone seguem o freedesktop icon naming spec e dependem do tema instalado.
- comportamento validado no GNOME (`notify-send 0.8.8`); outros daemons/desktops podem variar.
- o estado `Idle` sem transcrição (falha na transcrição, erro ao gravar, parada
  do serviço) envia uma notificação curta de limpeza, evitando que um OSD de
  "Escutando..." ou "Transcrevendo..." com `-t 0` fique preso na tela.

### Diagnostico pela interface

`run_doctor()` usa `doctor::doctor_report_text()` do core.

`run_paste_test()` usa `injector::paste_test_report()` do core.

A CLI continua sendo a ferramenta principal para diagnostico profundo:

```bash
cargo run -- doctor
cargo run -- paste-test
```

### Estrutura atual

```text
linux-voice-typer/
├── Cargo.toml
├── config.example.toml
├── config.toml              (gerado pelo setup; ignorado pelo git)
├── execute                  (atalho local: desktop, cli, doctor, setup, build)
├── README.md
├── TECHNICAL.md
├── src/
│   ├── main.rs              (CLI: setup, doctor, paste-test, run)
│   ├── lib.rs               (process_recording: WAV -> whisper -> paste)
│   ├── cli.rs               (definicao dos comandos e flags)
│   ├── config.rs            (Config, PasteBackend, PasteShortcut)
│   ├── fs_paths.rs          (AppPaths: config.toml, whisper.cpp, temp)
│   ├── platform.rs          (Linux/Wayland/package manager)
│   ├── error.rs             (AppError)
│   ├── audio.rs             (Recorder: cpal -> WAV 16 kHz mono)
│   ├── hotkey.rs            (keytap global + modo terminal)
│   ├── history.rs           (historico local em history.jsonl)
│   ├── transcriber.rs       (executa whisper-cli)
│   ├── clipboard.rs         (wl-clipboard ou xclip/xsel)
│   ├── injector.rs          (auto: wtype -> ydotool -> uinput)
│   ├── uinput.rs            (teclado virtual via /dev/uinput)
│   ├── libei.rs             (injecao via portal XDG / protocolo EI)
│   ├── notify.rs            (notificacoes OSD via notify-send)
│   ├── service.rs           (VoiceTyperService)
│   ├── doctor.rs            (diagnostico)
│   ├── setup.rs             (setup do whisper.cpp e modelo)
│   ├── systemd.rs           (unit systemd --user)
│   ├── downloader.rs        (execucao de comandos externos)
│   └── autostart.rs         (entrada .desktop em ~/.config/autostart)
└── apps/
    └── desktop/
        ├── src-tauri/       (Tauri v2: comandos, tray, janela)
        ├── src/             (frontend JS/CSS)
        ├── index.html
        ├── package.json
        └── vite.config.js
```

O crate principal ainda preserva `cargo run -- ...` para a CLI.

### Depuracao da interface

Para ver logs do frontend no console do navegador:

1. Abra a interface com `npm run tauri dev`
2. Clique com botao direito na janela -> Inspect (ou pressione F12)
3. Abra a aba Console

Para ver logs do backend Rust no terminal:

```bash
cd apps/desktop
npm run tauri dev 2>&1 | tee /tmp/tauri-debug.log
```

Logs uteis aparecem com `INFO`, `WARN` e `ERROR` do `env_logger`.

Para depurar erros de compilacao do backend:

```bash
cd apps/desktop
cargo check --manifest-path src-tauri/Cargo.toml
```

### Validacao da interface

Apos `npm run tauri dev`, a janela deve mostrar:

- Status do servico (Rodando/Parado)
- Hotkey atual
- Modo (Editor/Terminal)
- Backend (uinput, etc)
- Idioma
- Modelo Whisper (nome do arquivo, nao path completo)
- Whisper CLI (nome do arquivo)
- Microfone detectado
- Ultima transcricao

Botaoes e seus efeitos:

- **Iniciar / Pausar**: botao unico de alternancia (topo e aba Principal); inicia
  se estiver parado, para se estiver rodando
- **Modo Editor**: aplica `ctrl_v`, salva no config.toml
- **Modo Terminal**: aplica `ctrl_shift_v`, salva no config.toml
- **Testar paste**: executa `paste_test_report()` e mostra resultado no console
- **Rodar doctor**: executa `doctor_report_text()` e mostra resultado no console
- **Reiniciar Servico**: reinicia o loop do servico com a config atual
- **Atualizar Logs**: recarrega a lista de eventos do daemon
- **Copiar / Limpar**: copia ou limpa a saida do console de diagnostico
- **Salvar configuracoes**: grava config.toml e reinicia o servico se ele estiver rodando
- **Abrir config**: abre config.toml no editor do sistema
- **Gravar Tecla**: captura a tecla/combinacao pressionada e preenche o campo Hotkey

### Tray icon no GNOME

O tray icon usa `TrayIconBuilder` do Tauri v2 com o recurso `tray-icon`.

No GNOME, o tray icon pode nao aparecer por padrao. O GNOME nao fornece
suporte nativo a AppIndicator/KStatusNotifier. Para usar o tray icon:

1. Instale a extensao `AppIndicator and KStatusNotifierItem Support`:

```bash
# Ubuntu/Debian
sudo apt install gnome-shell-extension-appindicator

# Ou via Extensions GNOME (https://extensions.gnome.org)
```

2. Reinicie a sessao GNOME apos instalar a extensao.

3. Se o tray icon ainda nao aparecer, verifique se a extensao esta ativa:

```bash
gnome-extensions list | grep appindicator
gnome-extensions enable appindicatorsupport@rgcjonas.gmail.com
```

Sem a extensao, o app funciona normalmente pela janela. Como nao existe host
de tray, o fechamento da janela encerra o app (comportamento seguro) em vez de
escondê-lo; com a extensao ativa, fechar esconde para a bandeja (enquanto
`close_to_tray = true`).

### Erros amigaveis na interface

A interface traduz erros tecnicos do core para mensagens compreensiveis:

| Erro tecnico | Mensagem na interface |
|---|---|
| model_path does not exist | Modelo Whisper nao encontrado. Rode `cargo run -- setup` ou verifique o caminho em Configuracoes. |
| whisper_bin does not exist | Whisper CLI nao encontrado. Rode `cargo run -- setup` para instalar. |
| hotkey cannot be empty | Hotkey invalida. Verifique o campo Hotkey em Configuracoes. |
| audio error | Erro ao acessar o microfone. Verifique se ha um microfone disponivel. |
| hotkey error | Erro ao registrar hotkey global. Verifique se ha outro programa usando a mesma combinacao. |
| transcriber returned empty text | Whisper retornou texto vazio. Verifique o modelo e o audio gravado. |
| dependency missing | Dependencia ausente: <nome>. Rode `cargo run -- doctor` para diagnostico. |
| unsupported feature | Recurso nao disponivel: <detalhe>. |

## Troubleshooting

### Compositor does not support the virtual keyboard protocol

Isso aponta para limitacao do compositor Wayland com `wtype`.

Opcoes praticas:

1. configurar `paste_backend = "uinput"`
2. garantir `/dev/uinput` acessivel (`sudo modprobe uinput`)
3. usar `paste_shortcut = "ctrl_shift_v"` ou `cargo run -- run --terminal-paste` para terminais
4. se preferir daemon externo, usar `paste_backend = "ydotool"` e iniciar `ydotoold`
5. como ultimo fallback, usar `paste_backend = "none"` e colar manualmente

Mesmo quando o auto-paste falha, a transcricao continua no clipboard.

### Funciona no terminal, mas nao cola no editor

Terminais normalmente usam `Ctrl+Shift+V`. Editores e inputs normais usam
`Ctrl+V`.

Rode em modo editor:

```bash
cargo run -- run --editor-paste
```

Ou deixe no `config.toml`:

```toml
paste_shortcut = "ctrl_v"
```

### Funciona no editor, mas nao cola no terminal

Rode em modo terminal:

```bash
cargo run -- run --terminal-paste
```

Ou deixe no `config.toml`:

```toml
paste_shortcut = "ctrl_shift_v"
```

### Clipboard nao funciona

Se `wl-paste` nao retorna o texto copiado:

```bash
echo "teste" | wl-copy
wl-paste
```

Se isso nao funcionar, verifique se esta em uma sessao Wayland e se `wl-clipboard` esta instalado:

```bash
sudo apt install wl-clipboard
```

### ydotool nao cola

Se `ydotool` esta instalado mas nao cola:

1. Verifique se o daemon esta rodando:

```bash
pgrep ydotoold
```

2. Se nao estiver, inicie:

```bash
sudo modprobe uinput
sudo ydotoold
```

3. Teste manualmente:

```bash
echo "teste" | wl-copy
ydotool key 29:1 47:1 47:0 29:0
```

## Limitacoes atuais

- Wayland nao tem um padrao universal para hotkeys globais em todos os compositores.
- `wtype` depende do protocolo `virtual-keyboard`, que nem todo compositor expõe.
- `ydotool` depende de `ydotoold` e normalmente de `/dev/uinput`.
- A injecao inicial continua sendo `clipboard + atalho de colagem`.
- Alguns terminais preferem `Ctrl+Shift+V`.
- Tray icon no GNOME pode depender de suporte AppIndicator/KStatusNotifier.
- Nao ha streaming em tempo real.
- O backend `libei` nao entra no modo `auto` e a sessao do portal nao persiste entre reinicios do app.

## Proximos passos

Nao implementados ou ainda experimentais nesta fase:

- Persistencia da sessao libei entre reinicios (token/restore do portal) e validacao no KDE/Plasma
- Suporte melhor a GNOME/KDE Wayland
- Historico com busca/filtro (o historico basico ja existe)

## Nota

Este MVP nao usa Electron. A CLI continua sendo preservada como base de debug e uso avancado, enquanto a interface Tauri e uma camada visual experimental sobre o mesmo core Rust.
