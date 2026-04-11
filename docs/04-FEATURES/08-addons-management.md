# 08 - Addons Management

## Objetivo
Permitir que o usuario instale, remova e configure addons WASM com papeis isolados de Discover, Source e Legacy Search.

## Componentes principais
- Frontend: lexicon/src/components/AddonsView.tsx.
- Frontend hook: lexicon/src/hooks/useAddons.ts.
- Backend comandos: lexicon/src-tauri/src/commands/addons.rs.
- Runtime: lexicon/src-tauri/src/plugins/manager.rs.
- Contratos de addon: lexicon/src-tauri/wit/search-plugin.wit (legacy) e lexicon/src-tauri/wit/discover-source-plugin.wit (discover/source).

## Fluxo principal
1. Usuario abre aba Addons.
2. Usuario instala addon escolhendo arquivo .wasm no seletor nativo.
3. Backend copia o arquivo para app_data_dir/plugins e recarrega o PluginManager.
4. Runtime identifica papel do addon (discover, source, legacy_search) por id/configuracao.
5. Usuario edita configuracoes chave/valor por addon.
6. Backend persiste configuracoes em user_settings com prefixo addon::<id>::<chave>.
7. Discover/Source usam apenas plugins carregados do diretorio de runtime do usuario.

## Regras de negocio
- Addon fora do diretorio de runtime nao pode ser removido pela UI.
- Configuracao e generica por addon, sem modelo fixo por fonte.
- AddonDescriptor inclui role para a UI exibir o papel detectado.
- Fallback mock de busca existe somente em build de desenvolvimento.
- Em bootstrap inicial, .wasm de src-tauri/plugins/dist pode ser copiado para runtime quando arquivo ainda nao existir.

## Decisoes de arquitetura
- Integracoes de fontes externas no core foram removidas para cumprir isolamento por plugin.
- Contratos WIT separados permitem evolucao independente de Discover e Source.
- O host Rust oferece HTTP generico para addons WASM via interface host-http.

## Testes minimos
- Instalar addon .wasm valido pela aba Addons.
- Salvar configuracoes e validar leitura no ciclo Discover/Source.
- Remover addon e validar recarga sem resultados daquele plugin.
- Confirmar que em producao nao ha fallback mock quando todos addons falham.

## ⚠️ Inconsistências encontradas
- O bootstrap de plugins locais melhora primeiro uso, mas pode confundir expectativa de instalacao manual estrita em ambientes de desenvolvimento.
