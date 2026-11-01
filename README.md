# Monolito Greeter

Greeter **nativo em Rust + GTK4 para LightDM**, com entrada manual de contas
locais e Active Directory. Duas telas, paisagem procedural original, controles
de vidro inspirados no Raven e hierarquia visual inspirada nas referências
Apple/Windows. Não usa WebKit, navegador ou página HTML.

## Compilar e visualizar no Linux Mint 22

Requer Rust/Cargo **1.85+**, GTK **4.8+**, X11 e liblightdm-gobject **1.30+**.
No Mint 22, instale as bibliotecas e utilize um toolchain Rust que satisfaça
essa versão; confira `rustc --version` antes de compilar. O `Cargo.lock` faz
parte do projeto.

```sh
sudo apt-get install build-essential pkg-config libgtk-4-dev liblightdm-gobject-dev \
    dbus-x11 xvfb xauth x11-utils xserver-xephyr
cargo build --release --locked
cargo test --locked
./target/release/monolito-greeter --demo
```

A prévia abre em janela, mostra um aviso permanente e **não se conecta ao
LightDM**, não autentica usuários e não executa ações de energia. Use qualquer
nome, senha `monolito` e código de demonstração `000000`. Senhas diferentes
exercitam o estado de erro. `Escape` cancela a conversa ou volta à tela inicial.
Sem `--demo`, o processo exige um LightDM real e falha fechado se ele não estiver
disponível. Não existe fallback automático para o modo de demonstração.

## Testar sem substituir a tela de login

Em um desktop X11, abra um servidor aninhado:

```sh
Xephyr :2 -screen 1280x800 -nolisten tcp &
DISPLAY=:2 GDK_BACKEND=x11 ./target/release/monolito-greeter --demo
```

Para o teste automatizado sem desktop, com Xvfb no PATH:

```sh
sh scripts/smoke-test.sh target/release/monolito-greeter
```

Esse teste dirige os widgets GTK e o adaptador simulado: senha incorreta,
nova tentativa com `DOMINIO\usuario`, aviso ARCFOUR informativo e prompt de
confirmação, senha, segundo fator, sessão simulada e limpeza dos campos.
Ele não valida autenticação PAM real.

## Instalar e ativar no Mint

Teste a integração real primeiro numa **VM Mint com snapshot**, acesso a TTY
e o greeter padrão preservado. O instalador coloca o binário e o arquivo de
sessão, mas não muda o seat ativo nem reinicia o display manager:

```sh
sudo sh scripts/install.sh
# Verifique o empacotamento sem privilégios, se preferir:
# DESTDIR=/tmp/monolito-package sh scripts/install.sh
```

Na VM, para registrar uma instância de teste real, instale os arquivos e rode
`lightdm --test-mode --debug` como usuário sem privilégios, de um terminal da
sessão gráfica. O LightDM usa Xephyr no modo de teste. Caso sua distribuição
restrinja esse modo, utilize o seat da VM; não relaxe permissões de PAM.

Quando estiver pronto para testar no seat da VM:

```sh
sudo install -Dm644 data/50-monolito-greeter.conf \
    /etc/lightdm/lightdm.conf.d/50-monolito-greeter.conf
lightdm --show-config
```

Confirme que nenhuma configuração posterior sobrescreve `greeter-session`.
Encerre sua sessão e reinicie a VM. Para restaurar o greeter anterior por TTY,
remova **apenas** o arquivo `50-monolito-greeter.conf` criado acima e reinicie a
VM. Logs: `journalctl -u lightdm` e `/var/log/lightdm/`.

### Active Directory / PAM

O greeter encaminha o nome digitado ao LightDM: `DOMINIO\usuario`,
`usuario@dominio.com` ou `usuario`. A máquina precisa estar previamente integrada
ao domínio, com DNS/horário corretos e PAM/SSSD ou winbind configurados para o
serviço `lightdm`. O greeter não faz o ingresso no domínio nem modifica PAM.

Cada `show-prompt` recebe no máximo uma resposta. A senha inicial só é utilizada
uma vez, em um prompt de senha reconhecido. Desafios desconhecidos, OTP e troca
de senha permanecem interativos, respeitando se o PAM pede eco ou segredo.
Mensagens `show-message` nunca geram `respond()`.

A confirmação vazia automática exige **simultaneamente** um prompt não secreto,
menção a ARCFOUR/RC4, descontinuação e instrução explícita para pressionar Enter.
Ela é limitada a três confirmações por tentativa e pode ser desativada em
`config.ini`. Isso refina a correspondência ampla sugerida no documento do
Gemini: termos isolados como “encryption” ou “confirmar” não constituem
autorização para responder a desafios. O tratamento não habilita RC4 nem
altera a política criptográfica do domínio.

Senhas/respostas não são registradas em logs. Buffers Rust temporários usam
`Zeroizing`; os campos são apagados após envio/cancelamento. GTK e a biblioteca
LightDM gerenciam seus próprios buffers, sem garantia de apagamento integral
de todas as cópias de memória.

## Personalizar

Configuração: `/etc/monolito-greeter/config.ini`; exemplo em [data/config.ini](data/config.ini).
Para testar uma configuração sem instalar:

```sh
MONOLITO_CONFIG="$PWD/data/config.ini" ./target/release/monolito-greeter --demo
```

O wallpaper opcional precisa de caminho absoluto e permissão de leitura para
o usuário `lightdm`. Não use imagens no diretório privado de outro usuário.
Sem wallpaper, a paisagem é desenhada localmente em Cairo. Não há chamadas de
rede para fontes, imagens, clima ou telemetria. O menu de acessibilidade oferece
alto contraste, texto maior e redução de movimento. Sessões e capacidades de
energia vêm da liblightdm; ações de energia exigem confirmação.

Veja [docs/design.md](docs/design.md) para decisões visuais e limites do vidro
em GTK. Raven é referência visual, não uma dependência binária; o código GTK
é próprio. Esta versão não implementa leitor de tela/teclado virtual integrado,
troca de layout de teclado nem distribuição de telas por vários monitores.

## Ambiente de desenvolvimento na nuvem

No Debian 13 deste workspace, sem privilégios administrativos:

```sh
bash scripts/setup-cloud.sh
. /workspace/.monolito-native/env.sh
cargo build --locked
cargo test --locked
sh scripts/smoke-test.sh
```

O bootstrap usa pacotes Debian assinados e extrai as ferramentas em
`/workspace/.monolito-native`, fora do checkout. É necessário ativar `env.sh`
em novos terminais. Cada tarefa já é isolada: use o checkout existente e não
crie worktrees sem solicitação explícita.

## Limites de validação

A compilação, os testes da política PAM e a prévia gráfica podem ser validados
na nuvem. A abertura de uma sessão real, autenticação AD, ações de energia e
compatibilidade com a configuração específica do Mint exigem a VM/seat de
teste descritos acima. O projeto não deve ser considerado validado em produção
até esses testes serem concluídos.
