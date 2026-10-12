# Linux Voice Typer 🎙️⚡

> **Ditado por voz 100% local, ultra-rápido e privado para Linux (Wayland).**  
> Fale no microfone, solte o atalho e o texto é transcrito pelo Whisper local e colado automaticamente na janela ativa — seja no seu editor de código, navegador, terminal ou chat.

---

## 🌟 Principais Recursos

- 🧠 **Privacidade Total & Offline:** Todo o processamento de áudio é feito localmente na sua máquina utilizando Whisper.cpp. Sem envio de dados para servidores externos ou nuvem.
- ⚡ **Auto-Paste Inteligente no Wayland:** Backend nativo com teclado virtual de kernel (`uinput`) que resolve de ponta a ponta as limitações tradicionais de colagem no Wayland (compatível com GNOME, KDE, Hyprland, Sway, etc.). Backends disponíveis: `auto`, `uinput`, `ydotool`, `wtype`, `libei` (portal XDG, experimental) e `none`.
- 🎛️ **Interface Desktop Moderna em Abas:**
  - **Principal:** Controle de serviço em tempo real, alternador rápido de modo (Editor vs Terminal), visualizador de transcrição com cópia em 1 clique e cards de status.
  - **Configurações:** Painel completo para idioma, microfone de entrada, modelo Whisper, caminhos (com seletores nativos de arquivo), inicialização automática e preferências de injeção.
  - **Histórico:** Últimas 100 transcrições salvas localmente (`history.jsonl`), com cópia em 1 clique e limpeza.
  - **Diagnóstico & Debug:** Diagnóstico completo do sistema (`doctor`), teste de colagem automática (`paste-test`), console de comandos e logs de eventos do daemon em tempo real.
- 🟢🔴⚪ **System Tray Dinâmico (3 Estados Visuais):**
  - **🟢 Verde:** Serviço ativo e pronto para escutar o atalho.
  - **🔴 Vermelho:** Microfone gravando / escutando sua fala ativamente.
  - **⚪ Cinza:** Serviço pausado ou motor parado.
- 🌊 **Waveform Reativa Real:** Barra de ondas sonoras no HUD que reage dinamicamente ao volume e à intensidade real da sua voz (cálculo de amplitude RMS no backend Rust via `cpal`), sem loops falsos de CSS.
- ⏱️ **Cronômetro em Tempo Real:** Timer ao vivo durante a gravação e durante a transcrição do Whisper, mostrando exatamente quanto tempo foi falado e o tempo de processamento.
- 🔔 **Notificações OSD (interface desktop):** Avisos de desktop via `notify-send` para “Escutando...”, “Transcrevendo...” e “Ditado Colado!”, sincronizados com o estado do serviço. A CLI não envia notificações.
- ⌨️ **Gravador Interativo de Teclas:** Configure qualquer atalho ou tecla única (como `F7`, `F8`, `F12`, teclas macro) ou combinações (`Control+Alt+Space`, `Alt+Space`) apenas clicando em *Gravar Tecla* e pressionando-a no teclado.
- 🚀 **Autostart no Sistema & Inicialização Automática:**
  - Opção nativa para iniciar junto com o login da sessão Linux (`~/.config/autostart`).
  - Opção para o serviço de voz já inicializar ativo assim que o aplicativo for aberto.
- 💻 **CLI Robusta:** Pode ser executado 100% via linha de comando (configuração pelo `config.toml`) ou em modo de terminal puro (`--terminal-hotkey`).

---

## 🏗️ Como Funciona

```text
[Atalho Pressionado (ex: F7 ou Ctrl+Alt+Space)]
           │
           ▼
[Gravação de Áudio via cpal (RMS em tempo real no HUD)]
           │
           ▼
[Atalho Solto / Concluído]
           │
           ▼
[Transcrição de Alta Precisão via Whisper local (.bin)]
           │
           ▼
[Cópia do Texto para a Área de Transferência (Clipboard)]
           │
           ▼
[Injeção Automática via wtype / ydotool / uinput]
   ├── Modo Editor:    Ctrl + V (VS Code, Chrome, WhatsApp, Obsidian)
   └── Modo Terminal:  Ctrl + Shift + V (Ghostty, Alacritty, GNOME Terminal)
```

---

## 🚀 Instalação Rápida

### 1. Pré-requisitos (Ubuntu / Debian)

Certifique-se de ter os pacotes essenciais instalados:

```bash
sudo apt update
sudo apt install build-essential cmake git pkg-config libasound2-dev libssl-dev libglib2.0-dev libgtk-3-dev libayatana-appindicator3-dev
```

Para usar a colagem via `uinput` (recomendada no GNOME/Wayland e usada como fallback do backend `auto`), adicione seu usuário ao grupo `input`:

```bash
sudo usermod -aG input $USER
# Observação: Reinicie a sessão (logout/login) para aplicar a permissão de grupo.
```

> **Requisito de sessão:** o clipboard é detectado automaticamente — `wl-clipboard` no Wayland ou `xclip`/`xsel` no X11 (ex.: `sudo apt install xclip`). Sem nenhum deles a cópia/colagem falha; o `doctor` aponta o que instalar.

### 2. Clonar e Configurar o Projeto

```bash
git clone https://github.com/pabloapfarias/linux-typer.git
cd linux-typer

# Baixa e compila o Whisper.cpp e o modelo padrão (ggml-small.bin)
cargo run -- setup

# Valida o ambiente, microfone e permissões
cargo run -- doctor
```

---

## 🖥️ Executando a Interface Desktop

A interface gráfica foi construída com **Tauri v2** + **Vite**, oferecendo consumo mínimo de memória e integração nativa com o System Tray do Linux:

```bash
cd apps/desktop
npm install
npm run tauri dev
```

### Principais Ações na Interface:
- **Gravar Tecla de Atalho:** Acesse a aba **Configurações**, clique em **Gravar Tecla** e pressione a tecla desejada (ex: `F7`, `F8` ou `Control+Alt+Space`).
- **Alternar Modo de Colagem:** Na aba **Principal**, selecione entre **Editor & Navegador** (`Ctrl+V`) ou **Terminal & Shell** (`Ctrl+Shift+V`).
- **Escolher Microfone:** Na aba **Configurações**, selecione o dispositivo em **Microfone de Entrada** (ou mantenha "Padrão do Sistema").
- **Selecionar arquivos:** Use os botões **Selecionar…** ao lado dos campos de modelo/executável para abrir o seletor nativo de arquivos.
- **Histórico:** Na aba **Histórico**, clique em uma entrada para copiar o texto ou use **Limpar histórico**.
- **Iniciar com o Sistema:** Ative a caixa **Iniciar com o Sistema (Autostart)** para criar a entrada em `~/.config/autostart` e iniciar o aplicativo a cada login (com a opção *Iniciar Minimizado* ativa e tray disponível, a janela inicia oculta na bandeja).

---

## ⌨️ Executando via Linha de Comando (CLI)

Se preferir utilizar em segundo plano ou em ambientes sem interface web:

```bash
# Modo Editor padrão (Ctrl+V)
cargo run -- run --editor-paste

# Modo Terminal (Ctrl+Shift+V)
cargo run -- run --terminal-paste

# Modo Terminal Interativo (Pressione Enter para falar, Enter para parar)
cargo run -- run --terminal-hotkey --terminal-paste
```

Para gerar o binário de produção otimizado:

```bash
cargo build --release
./target/release/linux-voice-typer run
```

### Atalho com o script `execute`

O script `./execute` na raiz resume os comandos mais usados:

```bash
./execute             # interface desktop (Tauri + Vite)
./execute cli         # serviço CLI (modo editor padrão)
./execute interactive # modo terminal interativo (Enter para falar/parar)
./execute doctor      # diagnóstico do sistema
./execute setup       # baixa e compila whisper.cpp + modelo
./execute build       # binário release
./execute check       # verificação local: fmt + testes + clippy
```

### Serviço `systemd --user`

Para manter o ditado rodando como serviço do usuário (sem terminal aberto):

```bash
# Instala a unit, habilita e inicia
cargo run -- service install

# Apenas instala (sem habilitar)
cargo run -- service install --no-enable

# Status / remoção
cargo run -- service status
cargo run -- service uninstall
```

Se o Wayland não for herdado pelo gerenciador de serviços:

```bash
systemctl --user import-environment WAYLAND_DISPLAY DISPLAY XDG_SESSION_TYPE
systemctl --user restart linux-voice-typer
```

---

## ⚙️ Arquivo de Configuração (`config.toml`)

O arquivo de configuração fica no diretório do projeto (descoberto a partir do diretório atual; veja o modelo comentado em `config.example.toml`):

```toml
hotkey = "F7"
language = "pt"
model_path = "/home/USER/.local/share/linux-voice-typer/whisper.cpp/models/ggml-small.bin"
whisper_bin = "/home/USER/.local/share/linux-voice-typer/whisper.cpp/build/bin/whisper-cli"
insert_mode = "clipboard"
restore_clipboard = false
auto_paste = true
paste_backend = "auto"        # auto = wtype -> ydotool -> uinput -> clipboard ("libei" usa o portal XDG)
paste_shortcut = "ctrl_v"     # ctrl_v (editor) | ctrl_shift_v (terminal)
trim_text = true
temp_dir = "/home/USER/.local/share/linux-voice-typer/temp"
auto_start_service = true     # inicia o servico ao abrir o app
start_minimized = false       # inicia oculto na bandeja (se houver tray)
start_with_system = true      # entrada em ~/.config/autostart
close_to_tray = true          # fechar a janela esconde para a bandeja (se houver tray)
osd_enabled = true            # notificacoes de desktop (false desativa todo o OSD)
# input_device = "Nome do microfone"  # padrao do sistema se omitido
```

---

## 🩺 Diagnóstico e Resolução de Problemas

Se encontrar problemas de microfone ou simulação de teclas, execute o assistente de diagnóstico:

```bash
cargo run -- doctor
cargo run -- paste-test
```

O diagnóstico verifica automaticamente:
- Sistema Linux e sessão Wayland;
- Teste real de escrita/leitura do clipboard (`wl-copy`/`wl-paste`);
- Backends de colagem (`wtype`, `ydotool`, `ydotoold`) e permissão de escrita em `/dev/uinput` (com criação real de um teclado virtual);
- Existência do binário `whisper-cli`, do modelo de IA e escrita em `temp_dir`;
- Captura do microfone padrão via `cpal` (ALSA/PulseAudio/PipeWire);
- Inicialização do listener de hotkey global (`keytap`).

Consulte o documento técnico detalhado para mais informações:
📄 [**TECHNICAL.md**](TECHNICAL.md)

📌 [**PENDENCIAS.md**](PENDENCIAS.md) — bugs e melhorias pendentes, organizados por prioridade.

---

## 📄 Licença

Distribuído sob a licença [MIT](LICENSE).
