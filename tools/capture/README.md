# Captura do demo

Grava o app publicado com o som que ele mesmo gera, sem tomar a tela.

    ./record.sh out/master.mkv reel.mjs

Peças: `Xvfb` (display virtual, mantém o pipeline de áudio que o headless
descarta), `module-null-sink` do PipeWire (placa de som falsa, isola a captura),
`ffmpeg` com duas entradas num processo só (sem sincronia para alinhar depois) e
Playwright dirigindo o Chromium.

Variáveis: `CHORDZ_URL`, `CAPTURE_W`, `CAPTURE_H`, `CAPTURE_FPS`, `DISPLAY_NUM`.

Verificar uma gravação:

    ffmpeg -i out/master.mkv -af volumedetect -f null - 2>&1 | grep mean_volume

Acima de -50 dB tem som. Perto de -91 dB é silêncio digital.

Não é determinístico: o áudio é gerado em tempo real, e um engasgo da máquina
vira glitch. Regravar quando acontecer.

## Sobre o `node_modules` do driver

O `playwright` que o driver importa vive em `web/node_modules`. O
resolvedor de módulos ESM do node não honra `NODE_PATH` (isso só afeta
CommonJS) e também **não usa o cwd do processo** — ele sobe a árvore de
diretórios a partir do arquivo que faz o `import`, procurando um
`node_modules`. Como `tools/capture/` é irmão de `web/`, não ancestral,
nem variável de ambiente nem `cd` resolvem isso: só um `node_modules` que
exista ao lado do driver funciona.

Por isso `record.sh` cria (uma vez, se ainda não existir) um link simbólico
`tools/capture/node_modules -> ../../web/node_modules` antes de invocar o
driver. É autocurativo: se o link sumir ou `web/node_modules` for
reinstalado, a próxima chamada recria o link. Ele está no `.gitignore`
(assim como `web/node_modules/`) porque é derivado, não versionado.

Ou seja, tanto faz rodar

    ./tools/capture/record.sh out/master.mkv tools/capture/reel.mjs

da raiz do repo quanto de qualquer outro diretório: `OUT` e `DRIVER` são
resolvidos para caminhos absolutos antes de qualquer coisa.

## Armadilha: vídeo preto

Todo driver precisa passar `--ozone-platform=x11` ao Chromium. Sem isso, numa
máquina cujo desktop roda Wayland, o Chromium detecta a sessão Wayland real em
vez do display do Xvfb: os cliques funcionam, o áudio sai certo, e a captura
grava um quadro totalmente preto.

A falha é silenciosa para qualquer verificação de áudio, então confirme a
imagem extraindo um quadro:

    ffmpeg -ss 5 -i out/master.mkv -frames:v 1 /tmp/f.png

Duas gravações desta série (o smoke inicial e o probe de andamento) saíram
pretas por causa disso e ninguém percebeu, porque só o som tinha sido medido.
