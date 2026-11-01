# Direção visual

Monolito Greeter combina a hierarquia das telas de entrada fornecidas com os
materiais e controles descritos em `docs/THEMES.md` e `examples/glass/glass.xml`
do [Raven](https://github.com/oscar-soirey/Raven), consultado no commit
`ff35f641ff95ef21777015540278c9d2de7172a9`.

- Relógio de grande escala, paisagem serena e transição para credenciais.
- Cápsulas de vidro para ações flutuantes; campos opacos para legibilidade.
- Card com raio de 28 px, controles de 12 px, contorno fino e sombras suaves.
- Acento mineral, tipografia leve e avatar contido, sem logotipos de terceiros.
- Navegação por teclado, foco visível, contraste elevado e movimento reduzido.

O Raven usa renderização própria em C/Vulkan. Esta implementação recria os
padrões em widgets GTK4 para cumprir a stack solicitada, manter acessibilidade
nativa e não carregar uma segunda árvore gráfica no processo de login. Não
inclui código, fontes ou imagens do Raven nem componentes binários da Apple.

GTK CSS não implementa `backdrop-filter` nem `acrylic-blur` de CSS web. O material
é composto por transparência, gradientes, borda e sombra. A arte procedural já
é suave; wallpapers externos podem ser pré-desfocados pelo administrador.
Isso evita depender de um compositor na sessão do LightDM.

A interface não enumera contas locais: a entrada manual serve tanto a usuários
locais quanto a AD. Não apresenta status de Wi-Fi, bateria ou idioma fictícios.
O layout de teclado é o configurado no sistema/seat; não há seletor de layout
nesta versão. A interface e a data são em português, com horário local do host.
