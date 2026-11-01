# Direção Visual e Design System

O **Monolito Greeter** é projetado como uma interface corporativa, sóbria e nativa em Rust + GTK4 para o LightDM (com foco em ambientes Linux XFCE/Cinnamon/Mint), combinando os formatos e a ergonomia do **Windows 11** com os materiais de **liquid glass / acrílico translúcido do macOS**.

## 1. Princípios de Design

- **Postura Corporativa e Direta:** Eliminação total de clichês de IA e textos excessivos (sem "Bem-vindo de volta", "Seu espaço", etc.). A interface vai direto ao ponto: identificação do usuário/domínio e campos de credencial.
- **Autenticação Direta de Etapa Única:** Login direto por usuário e senha (`DOMINIO\usuario`, `usuario@empresa.com` ou `usuario`), compatível com Active Directory (SSSD/winbind) e contas locais PAM, sem códigos secundários artificiais.
- **Ergonomia Windows 11:**
  - *Tela de Bloqueio:* Relógio de grande escala centralizado no terço superior com data em português, ícones de status/utilidades e transição fluida ao toque ou pressionar de qualquer tecla.
  - *Tela de Login:* Cartão centralizado com avatar circular generoso (86px), título do usuário/conta, dica de domínio, campos com botão de envio integrado (`→`) e alternador de contas corporativas no canto inferior esquerdo.
  - *Controles do Sistema:* Seletor de sessão do desktop (XFCE, Cinnamon, etc.), acessibilidade e menu de energia (Suspender, Reiniciar, Desligar) no canto inferior direito.
- **Materiais Liquid Glass (macOS):**
  - Superfícies translúcidas em camadas profundas (`alpha(#0c1524, 0.72)`).
  - Bordas nítidas de 1px (`alpha(white, 0.15)`) com realce de reflexo interno no topo (`inset 0 1px 0 alpha(white, 0.18)`).
  - Sombras suaves de elevação em múltiplos estágios para profundidade autêntica sem necessidade de compositores pesados.
- **Arte Procedural Cairo (Windows 11 Bloom):**
  - Fundo padrão vetorial procedural com curvas de fita e pétalas fluidas em tons de safira, cobalto e azul noturno profundo.
  - Suporte a wallpapers externos (arquivos estáticos em disco) através de `config.ini`.

## 2. Acessibilidade e Teclado

- **Navegação completa por teclado:** `Enter` no usuário foca a senha; `Enter` na senha ou no botão `→` dispara o login; `Escape` retorna à tela de bloqueio.
- **Modos de Acessibilidade:**
  - *Alto Contraste:* Fundos pretos sólidos, bordas brancas de 2px e tipografia de contraste máximo.
  - *Texto Maior:* Ajuste proporcional de tipografia e campos.
  - *Reduzir Movimento:* Desativa transições e animações para conformidade com preferências de acessibilidade motora e vestibular.
