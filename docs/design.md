# Direção Visual e Design System

O **Monolito Greeter** é projetado como uma interface corporativa, sóbria e nativa em Rust + GTK4 para o LightDM (com foco em ambientes Linux XFCE/Mint), seguindo com fidelidade a ergonomia e os formatos do **Windows 11**.

## 1. Princípios de Design

- **Zero AI-Slop e Sem Cards Supérfluos:** O formulário de login flutua diretamente sobre o fundo suavemente desfocado (*Gaussian blur*), sem cards artificiais, bordas espessas ou molduras de acrílico visíveis.
- **Identificação da Máquina:** O hostname do sistema é exibido de forma discreta no canto superior esquerdo, sem logotipos ou textos promocionais.
- **Usuário Prévio Selecionado (Padrão Windows 11):**
  - O greeter inicia com o usuário principal/último usuário já selecionado, exibindo seu avatar e apenas o campo de senha.
  - No canto inferior esquerdo é disponibilizado o alternador para "Outro usuário" (login manual com campo de usuário e senha para contas locais ou Active Directory).
- **Avatar Nativo Embutido:** O ícone circular de perfil oficial (silhueta cinza clássica em alta resolução) vem embutido no binário, com transparência alfa e sem falhas de tema de ícones.
- **Botão de Envio Discreto:** Botão `→` sóbrio e plano ao lado da senha, sem brilhos (*glow*), gradientes chamativos ou sombras pesadas.
- **Ambiente de Desktop Configurado:** A sessão padrão é o **XFCE**, definida via `config.ini` (`default-session=xfce`), sem poluir a interface visual com menus suspensos desnecessários.
- **Fundo Procedural em Cairo com Foco Suave:** Renderização vetorial nativa simulando desfoque gaussiano em tons profundos de azul noturno e safira, com vinheta suave nos cantos.

## 2. Acessibilidade e Teclado

- **Navegação completa por teclado:** `Enter` na senha ou no botão `→` dispara a autenticação; `Escape` retorna à tela de bloqueio.
- **Modos de Acessibilidade:**
  - *Alto Contraste:* Fundos pretos sólidos, bordas brancas e tipografia de contraste máximo.
  - *Texto Maior:* Ajuste proporcional de tipografia e campos.
  - *Reduzir Movimento:* Desativa transições para conformidade com preferências de acessibilidade.
