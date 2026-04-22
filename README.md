# 📚 Nossateca

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Built with Tauri](https://img.shields.io/badge/Built%20with-Tauri-39f)](https://tauri.app/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0+-blue)](https://www.typescriptlang.org/)

**Nossateca** é um gerenciador de biblioteca digital e leitor de livros para desktop. Leia EPUB, mangá em CBZ, registre anotações locais e descubra novos conteúdos através de um sistema extensível de plugins — tudo com seus dados armazenados localmente.

## 📋 Índice

- [Sobre](#-sobre-o-projeto)
- [Funcionalidades](#-funcionalidades)
- [Tecnologias](#-tecnologias-utilizadas)
- [Pré-requisitos](#-pré-requisitos)
- [Instalação](#-instalação-e-execução)
- [Desenvolvimento](#-desenvolvimento)
- [Estrutura do Projeto](#-estrutura-do-projeto)
- [Plugins](#-plugins)
- [Banco de Dados](#-banco-de-dados)
- [Roadmap](#-roadmap)
- [Contribuindo](#-contribuindo)
- [Licença](#-licença)


## 🎯 Sobre o Projeto

Nossateca resolve o problema de **centralizar sua biblioteca pessoal sem depender de serviços na nuvem**. Com foco em controle total dos dados e leitura offline, oferece:

- **Leitura com progresso persistente** — continue de onde parou
- **Anotações por trecho** — registre pensamentos enquanto lê
- **Descoberta extensível** — integre catálogos através de plugins
- **Biblioteca local** — seus dados nunca saem do seu dispositivo
- **Suporte multi-formato** — EPUB, PDF e mangá em CBZ

## ✨ Funcionalidades

### Core
- [x] **Importação de livros** — adicione EPUB, PDF e CBZ via seletor de arquivo
- [x] **Biblioteca organizada** — listagem, filtros e busca
- [x] **Leitor EPUB/PDF** — renderização de capítulos com navegação fluida
- [x] **Progresso persistente** — salva automaticamente sua posição
- [x] **Anotações e destaques** — CRUD completo com sincronização local
- [x] **Mangá em CBZ** — leitura com layout adaptado

### Discover & Plugins
- [x] **Sistema de plugins** — estenda funcionalidade com Discover e Source plugins
- [x] **Busca distribuída** — fan-out paralelo entre múltiplas fontes
- [x] **Descoberta de catálogos** — integre APIs externas isoladamente
- [x] **Gerenciador de downloads** — organize conteúdo baixado


---

## 🛠️ Tecnologias Utilizadas

| Categoria | Tecnologia |
|-----------|-----------|
| **Framework Desktop** | [Tauri 2](https://tauri.app/) |
| **Frontend** | [TypeScript](https://www.typescriptlang.org/), [Vite](https://vitejs.dev/) |
| **Backend** | [Rust](https://www.rust-lang.org/) |
| **Banco de Dados** | [SQLite](https://www.sqlite.org/) |
| **Build** | Vite, TypeScript Compiler |
| **Gerenciamento de Plugins** | Sistema nativo Tauri |


## ⚙️ Pré-requisitos

- **Node.js** >= 18
- **Rust** >= 1.70 (para compilar backend Tauri)
- **npm** ou **pnpm**
- **Sistema Operacional:** Windows, macOS ou Linux

## 🚀 Instalação e Execução

### 1. Clonar o Repositório

```bash
git clone https://github.com/AngelPolicarpo/nossateca.git
cd nossateca
```

### 2. Instalar Dependências

```bash
npm install
```

### 3. Executar em Desenvolvimento

```bash
# Inicia o app com hot-reload
npm run tauri:dev
```

O Tauri abrirá uma janela desktop. Vite compila o frontend e Tauri fornece o runtime.

### 4. Compilar para Produção

```bash
# Build frontend + backend
npm run tauri:build
```

O executável será gerado em `src-tauri/target/release/`.

---

### Convenções

- **Backend:** Comandos em `src-tauri/src/commands/`
- **Frontend:** Componentes em `lexicon/src/components/`
- **Tipos:** TypeScript em `lexicon/` e Rust em `src-tauri/`
- **Migrations:** Versionadas em `src-tauri/migrations/`

## 🤝 Contribuindo

Contribuições são bem-vindas! Por favor:

1. Faça um **Fork** do repositório
2. Crie uma branch: `git checkout -b feature/sua-funcionalidade`
3. Commit suas mudanças: `git commit -m 'feat: sua funcionalidade'`
4. Push: `git push origin feature/sua-funcionalidade`
5. Abra um **Pull Request**

### Diretrizes

- Siga [Conventional Commits](https://www.conventionalcommits.org/)
- Adicione testes para funcionalidades novas
- Atualize documentação em `docs/`
- Respeite o estilo de código existente
