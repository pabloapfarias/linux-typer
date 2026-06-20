# linux-voice-typer — Documentacao tecnica

`linux-voice-typer` e um MVP Linux-first, sem interface grafica, para ditado local com Whisper e colagem automatica no input ativo.

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

Com binario buildado:

```bash
cargo build --release
./target/release/linux-voice-typer setup
./target/release/linux-voice-typer doctor
./target/release/linux-voice-typer paste-test
./target/release/linux-voice-typer run
./target/release/linux-voice-typer run --editor-paste
./target/release/linux-voice-typer run --terminal-paste
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

# Modo de hotkey por terminal: Enter inicia e Enter para a gravacao
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
6. baixa um modelo Whisper automaticamente
7. cria ou atualiza `config.toml` com caminhos reais
8. cria `config.toml.bak` antes de sobrescrever um config existente

## O que o doctor verifica

- Linux
- Wayland
- `wl-copy`
- `wl-paste`
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

## Interface desktop / Tauri

A primeira interface desktop fica em:

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
npm run tauri -- build
```

O empacotamento pode exigir ferramentas adicionais do sistema, dependendo do
target (`deb`, `appimage`) e da distribuicao.

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
get_recent_logs()
set_mode_editor()
set_mode_terminal()
```

`select_model_file()` e `select_whisper_bin()` existem como pontos de extensao,
mas a primeira versao ainda usa campos de texto para os caminhos.

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
last_transcript()
status()
recent_events()
```

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
- `Restart` existe no backend para uso futuro ou por extensoes da UI.

Se a configuracao mudar enquanto o serviço está rodando, salve e reinicie o
serviço para garantir que a thread use os novos valores.

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
none
auto
```

Recomendado no GNOME/Wayland:

```toml
paste_backend = "uinput"
restore_clipboard = false
```

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

- fechar a janela esconde o app
- o serviço pode continuar rodando pelo tray
- `Abrir Configurações` mostra a janela novamente
- `Sair` encerra o app de verdade

No GNOME, a exibição do tray/AppIndicator pode depender de suporte do ambiente
ou de extensão compatível com AppIndicator/KStatusNotifier.

### Logs e eventos visuais

O serviço mantém os últimos eventos em memória. Exemplos:

```text
14:06 Serviço iniciado
14:06 Hotkey global ativa
14:16 Gravação iniciada
14:16 Gravação finalizada
14:16 Transcrição concluída
14:16 Texto colado com Ctrl+V
```

A tela mostra eventos resumidos. Logs técnicos detalhados continuam mais úteis
pelo console e pela CLI.

### Diagnostico pela interface

`run_doctor()` usa `doctor::doctor_report_text()` do core.

`run_paste_test()` usa `injector::paste_test_report()` do core.

A CLI continua sendo a ferramenta principal para diagnostico profundo:

```bash
cargo run -- doctor
cargo run -- paste-test
```

### Estrutura atual

Foi adotada a abordagem incremental:

```text
linux-voice-typer/
├── src/lib.rs
├── src/main.rs
├── src/service.rs
├── apps/
│   └── desktop/
│       ├── src-tauri/
│       ├── index.html
│       ├── package.json
│       └── src/
├── README.md
└── TECHNICAL.md
```

O crate principal ainda preserva `cargo run -- ...` para a CLI.

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
- Nao ha escolha de microfone por CLI ainda.

## Proximos passos

Nao implementados ou ainda experimentais nesta fase:

- Historico de transcricoes
- Escolha de microfone
- Escolha de modelo por seletor nativo de arquivos
- Suporte a `libei` / portal
- Suporte melhor a GNOME/KDE Wayland
- Empacotamento `.deb` / `AppImage`
- Servico `systemd --user`

## Nota

Este MVP nao usa Electron. A CLI continua sendo preservada como base de debug e uso avancado, enquanto a interface Tauri e uma camada visual experimental sobre o mesmo core Rust.
