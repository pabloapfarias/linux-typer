# linux-voice-typer

Ditado por voz local para Linux. Fale, solte o atalho e o texto aparece no
editor, navegador, terminal ou input ativo.

O `linux-voice-typer` usa Whisper localmente, copia a transcricao para o
clipboard e envia o comando de colagem correto para a janela ativa. A proposta
e simples: transformar fala em texto sem depender de uma interface grafica
pesada e sem mandar audio para servicos externos.

## Por que existe

Digitar muito em chats, editores, terminais e ferramentas de desenvolvimento
quebra o fluxo de trabalho. Ditado por voz ajuda, mas no Linux o auto-paste em
Wayland costuma falhar por diferencas entre compositores.

Este projeto resolve esse caminho de ponta a ponta:

- grava o microfone enquanto o atalho esta ativo
- transcreve localmente com Whisper
- copia o resultado para o clipboard
- cola automaticamente no alvo certo
- suporta editor comum e terminal Linux

## Principais recursos

- **Transcricao local:** audio processado na maquina com Whisper.
- **Sem interface pesada:** CLI simples, focado em velocidade e confiabilidade.
- **Auto-paste no Wayland:** backend `uinput` para contornar limitacoes do `wtype`.
- **Modo editor:** envia `Ctrl+V` para inputs comuns.
- **Modo terminal:** envia `Ctrl+Shift+V` para terminais, incluindo Codex CLI.
- **Interface desktop experimental:** janela visual com status, configuracoes e tray icon.
- **Fallback seguro:** mesmo se o auto-paste falhar, o texto permanece no clipboard.

## Uso rapido

Instale e valide o ambiente:

```bash
cargo run -- setup
cargo run -- doctor
```

Use em editores, navegadores e inputs comuns:

```bash
cargo run -- run --editor-paste
```

Use em terminais Linux:

```bash
cargo run -- run --terminal-paste
```

Use o modo de hotkey pelo proprio terminal:

```bash
cargo run -- run --terminal-hotkey --terminal-paste
```

Nesse modo, pressione `Enter` para iniciar a gravacao e `Enter` novamente para
parar, transcrever e colar.

## Interface desktop

Além da CLI, o projeto possui uma interface desktop experimental com tray icon
para iniciar/parar o serviço, escolher modo Editor/Terminal e acessar
diagnósticos.

```bash
cd apps/desktop
npm install
npm run tauri dev
```

No GNOME, o tray icon pode depender de suporte AppIndicator/KStatusNotifier.
Veja detalhes em [TECHNICAL.md](TECHNICAL.md).

Para detalhes técnicos, veja [TECHNICAL.md](TECHNICAL.md).

## Como funciona

```text
atalho pressionado
  -> grava audio do microfone
  -> salva WAV temporario
  -> transcreve com Whisper local
  -> copia texto para o clipboard
  -> envia Ctrl+V ou Ctrl+Shift+V
```

No Wayland, o projeto usa um backend de colagem compatível com uso diário em
editores e terminais. Os detalhes ficam em [TECHNICAL.md](TECHNICAL.md).

## Para quem e

- desenvolvedores que usam terminal e editores o dia todo
- usuarios Linux que querem ditado local sem solucao cloud
- quem trabalha com Codex CLI, chats, IDEs, notas e formularios
- quem precisa alternar entre editor comum e terminal sem perder o texto

## Documentacao tecnica

Detalhes de instalacao, configuracao, diagnostico, backends de paste e
troubleshooting ficam no documento tecnico:

[`TECHNICAL.md`](TECHNICAL.md)

## Status

Este projeto ainda e um MVP, mas ja valida o fluxo principal:

- gravacao local
- transcricao local
- clipboard
- auto-paste em editor
- auto-paste em terminal
- fallback para Wayland via `uinput`
- interface desktop experimental com tray icon

## Proximos passos

- historico de transcricoes
- selecao de microfone
- selecao de modelo Whisper
- melhorias de empacotamento desktop
- empacotamento `.deb` ou `AppImage`
- servico `systemd --user`

## Licenca

Defina a licenca antes de publicar o repositorio como projeto aberto.
