> ## Documentation Index
> Fetch the complete documentation index at: https://code.claude.com/docs/llms.txt
> Use this file to discover all available pages before exploring further.

# プラグインマーケットプレイスの作成と配布

> Claude Code拡張機能を配布するためのプラグインマーケットプレイスを構築およびホストします。

プラグインマーケットプレイスは、他のユーザーにプラグインを配布できるカタログです。マーケットプレイスは、一元化された検出、バージョン追跡、自動更新、および複数のソースタイプ（Gitリポジトリ、ローカルパスなど）のサポートを提供します。このガイドでは、チームやコミュニティとプラグインを共有するための独自のマーケットプレイスを作成する方法を説明します。

既存のマーケットプレイスからプラグインをインストールしたいですか？[既成プラグインの検出とインストール](/ja/discover-plugins)を参照してください。

## 概要

マーケットプレイスの作成と配布には以下が含まれます：

1. **プラグインの作成**：コマンド、エージェント、フック、MCPサーバー、またはLSPサーバーを使用して1つ以上のプラグインを構築します。このガイドでは、配布するプラグインが既にあることを前提としています。プラグインの作成方法の詳細については、[プラグインの作成](/ja/plugins)を参照してください。
2. **マーケットプレイスファイルの作成**：プラグインとその場所をリストする`marketplace.json`を定義します（[マーケットプレイスファイルの作成](#create-the-marketplace-file)を参照）。
3. **マーケットプレイスのホスト**：GitHub、GitLab、または別のGitホストにプッシュします（[マーケットプレイスのホストと配布](#host-and-distribute-marketplaces)を参照）。
4. **ユーザーと共有**：ユーザーは`/plugin marketplace add`でマーケットプレイスを追加し、個別のプラグインをインストールします（[プラグインの検出とインストール](/ja/discover-plugins)を参照）。

マーケットプレイスがライブになったら、リポジトリに変更をプッシュして更新できます。ユーザーは`/plugin marketplace update`でローカルコピーを更新します。

## チュートリアル：ローカルマーケットプレイスの作成

この例では、1つのプラグイン（コードレビュー用の`/review`スキル）を含むマーケットプレイスを作成します。ディレクトリ構造を作成し、スキルを追加し、プラグインマニフェストとマーケットプレイスカタログを作成してから、インストールしてテストします。

<Steps>
  <Step title="ディレクトリ構造の作成">
    ```bash  theme={null}
    mkdir -p my-marketplace/.claude-plugin
    mkdir -p my-marketplace/plugins/review-plugin/.claude-plugin
    mkdir -p my-marketplace/plugins/review-plugin/skills/review
    ```
  </Step>

  <Step title="スキルの作成">
    `/review`スキルが何をするかを定義する`SKILL.md`ファイルを作成します。

    ```markdown my-marketplace/plugins/review-plugin/skills/review/SKILL.md theme={null}
    ---
    description: Review code for bugs, security, and performance
    disable-model-invocation: true
    ---

    Review the code I've selected or the recent changes for:
    - Potential bugs or edge cases
    - Security concerns
    - Performance issues
    - Readability improvements

    Be concise and actionable.
    ```
  </Step>

  <Step title="プラグインマニフェストの作成">
    プラグインを説明する`plugin.json`ファイルを作成します。マニフェストは`.claude-plugin/`ディレクトリに配置されます。

    ```json my-marketplace/plugins/review-plugin/.claude-plugin/plugin.json theme={null}
    {
      "name": "review-plugin",
      "description": "Adds a /review skill for quick code reviews",
      "version": "1.0.0"
    }
    ```
  </Step>

  <Step title="マーケットプレイスファイルの作成">
    プラグインをリストするマーケットプレイスカタログを作成します。

    ```json my-marketplace/.claude-plugin/marketplace.json theme={null}
    {
      "name": "my-plugins",
      "owner": {
        "name": "Your Name"
      },
      "plugins": [
        {
          "name": "review-plugin",
          "source": "./plugins/review-plugin",
          "description": "Adds a /review skill for quick code reviews"
        }
      ]
    }
    ```
  </Step>

  <Step title="追加とインストール">
    マーケットプレイスを追加してプラグインをインストールします。

    ```shell  theme={null}
    /plugin marketplace add ./my-marketplace
    /plugin install review-plugin@my-plugins
    ```
  </Step>

  <Step title="試してみる">
    エディタでコードを選択して、新しいコマンドを実行します。

    ```shell  theme={null}
    /review
    ```
  </Step>
</Steps>

プラグインができることの詳細（フック、エージェント、MCPサーバー、LSPサーバーを含む）については、[プラグイン](/ja/plugins)を参照してください。

<Note>
  **プラグインのインストール方法**：ユーザーがプラグインをインストールすると、Claude Codeはプラグインディレクトリをキャッシュロケーションにコピーします。これは、`../shared-utils`のようなパスを使用してプラグインディレクトリの外のファイルを参照できないことを意味します。これらのファイルはコピーされないためです。

  複数のプラグイン間でファイルを共有する必要がある場合は、シンボリックリンク（コピー中にフォローされる）を使用するか、マーケットプレイスを再構成して、共有ディレクトリがプラグインソースパス内にあるようにします。詳細については、[プラグインキャッシングとファイル解決](/ja/plugins-reference#plugin-caching-and-file-resolution)を参照してください。
</Note>

## マーケットプレイスファイルの作成

リポジトリルートに`.claude-plugin/marketplace.json`を作成します。このファイルは、マーケットプレイスの名前、所有者情報、およびソース付きプラグインのリストを定義します。

各プラグインエントリには、最低限`name`と`source`（取得元）が必要です。利用可能なすべてのフィールドについては、以下の[完全なスキーマ](#marketplace-schema)を参照してください。

```json  theme={null}
{
  "name": "company-tools",
  "owner": {
    "name": "DevTools Team",
    "email": "devtools@example.com"
  },
  "plugins": [
    {
      "name": "code-formatter",
      "source": "./plugins/formatter",
      "description": "Automatic code formatting on save",
      "version": "2.1.0",
      "author": {
        "name": "DevTools Team"
      }
    },
    {
      "name": "deployment-tools",
      "source": {
        "source": "github",
        "repo": "company/deploy-plugin"
      },
      "description": "Deployment automation tools"
    }
  ]
}
```

## マーケットプレイススキーマ

### 必須フィールド

| フィールド     | タイプ    | 説明                                                                                                                | 例              |
| :-------- | :----- | :---------------------------------------------------------------------------------------------------------------- | :------------- |
| `name`    | 文字列    | マーケットプレイス識別子（ケバブケース、スペースなし）。これは公開向けです：ユーザーはプラグインをインストールするときにこれを見ます（例：`/plugin install my-tool@your-marketplace`）。 | `"acme-tools"` |
| `owner`   | オブジェクト | マーケットプレイスメンテナー情報（[以下のフィールドを参照](#owner-fields)）                                                                    |                |
| `plugins` | 配列     | 利用可能なプラグインのリスト                                                                                                    | 以下を参照          |

<Note>
  **予約名**：以下のマーケットプレイス名はAnthropicの公式使用のために予約されており、サードパーティのマーケットプレイスでは使用できません：`claude-code-marketplace`、`claude-code-plugins`、`claude-plugins-official`、`anthropic-marketplace`、`anthropic-plugins`、`agent-skills`、`life-sciences`。公式マーケットプレイスになりすましている名前（`official-claude-plugins`や`anthropic-tools-v2`など）もブロックされています。
</Note>

### 所有者フィールド

| フィールド   | タイプ | 必須  | 説明             |
| :------ | :-- | :-- | :------------- |
| `name`  | 文字列 | はい  | メンテナーまたはチームの名前 |
| `email` | 文字列 | いいえ | メンテナーの連絡先メール   |

### オプションメタデータ

| フィールド                  | タイプ | 説明                                                                                                                           |
| :--------------------- | :-- | :--------------------------------------------------------------------------------------------------------------------------- |
| `metadata.description` | 文字列 | マーケットプレイスの簡潔な説明                                                                                                              |
| `metadata.version`     | 文字列 | マーケットプレイスバージョン                                                                                                               |
| `metadata.pluginRoot`  | 文字列 | 相対プラグインソースパスの前に付加されるベースディレクトリ（例：`"./plugins"`を使用すると、`"source": "./plugins/formatter"`の代わりに`"source": "formatter"`と書くことができます） |

## プラグインエントリ

`plugins`配列内の各プラグインエントリは、プラグインとその場所を説明します。[プラグインマニフェストスキーマ](/ja/plugins-reference#plugin-manifest-schema)の任意のフィールド（`description`、`version`、`author`、`commands`、`hooks`など）を含めることができます。また、これらのマーケットプレイス固有のフィールド：`source`、`category`、`tags`、および`strict`も含めることができます。

### 必須フィールド

| フィールド    | タイプ         | 説明                                                                                                |
| :------- | :---------- | :------------------------------------------------------------------------------------------------ |
| `name`   | 文字列         | プラグイン識別子（ケバブケース、スペースなし）。これは公開向けです：ユーザーはインストール時にこれを見ます（例：`/plugin install my-plugin@marketplace`）。 |
| `source` | 文字列\|オブジェクト | プラグインを取得する場所（以下の[プラグインソース](#plugin-sources)を参照）                                                   |

### オプションプラグインフィールド

**標準メタデータフィールド：**

| フィールド         | タイプ    | 説明                                                                                                                                                                                                                                                                 |
| :------------ | :----- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `description` | 文字列    | プラグインの簡潔な説明                                                                                                                                                                                                                                                        |
| `version`     | 文字列    | プラグインバージョン                                                                                                                                                                                                                                                         |
| `author`      | オブジェクト | プラグイン作成者情報（`name`は必須、`email`はオプション）                                                                                                                                                                                                                                |
| `homepage`    | 文字列    | プラグインホームページまたはドキュメンテーションURL                                                                                                                                                                                                                                        |
| `repository`  | 文字列    | ソースコードリポジトリURL                                                                                                                                                                                                                                                     |
| `license`     | 文字列    | SPDXライセンス識別子（例：MIT、Apache-2.0）                                                                                                                                                                                                                                     |
| `keywords`    | 配列     | プラグイン検出とカテゴリ化用のタグ                                                                                                                                                                                                                                                  |
| `category`    | 文字列    | 組織用のプラグインカテゴリ                                                                                                                                                                                                                                                      |
| `tags`        | 配列     | 検索可能性用のタグ                                                                                                                                                                                                                                                          |
| `strict`      | ブール値   | プラグインが独自の`plugin.json`ファイルを必要とするかどうかを制御します。`true`（デフォルト）の場合、プラグインソースは`plugin.json`を含む必要があり、ここのマーケットプレイスエントリに追加したフィールドはそれとマージされます。`false`の場合、プラグインは独自の`plugin.json`を必要としません。マーケットプレイスエントリ自体がプラグインについてのすべてを定義します。マーケットプレイスファイル内で完全にシンプルなプラグインを定義したい場合は`false`を使用します。 |

**コンポーネント設定フィールド：**

| フィールド        | タイプ         | 説明                        |
| :----------- | :---------- | :------------------------ |
| `commands`   | 文字列\|配列     | コマンドファイルまたはディレクトリへのカスタムパス |
| `agents`     | 文字列\|配列     | エージェントファイルへのカスタムパス        |
| `hooks`      | 文字列\|オブジェクト | カスタムフック設定またはフックファイルへのパス   |
| `mcpServers` | 文字列\|オブジェクト | MCPサーバー設定またはMCP設定ファイルへのパス |
| `lspServers` | 文字列\|オブジェクト | LSPサーバー設定またはLSP設定ファイルへのパス |

## プラグインソース

### 相対パス

同じリポジトリ内のプラグインの場合：

```json  theme={null}
{
  "name": "my-plugin",
  "source": "./plugins/my-plugin"
}
```

<Note>
  相対パスは、ユーザーがGit（GitHub、GitLab、またはGit URL）経由でマーケットプレイスを追加する場合にのみ機能します。ユーザーが`marketplace.json`ファイルへの直接URLを介してマーケットプレイスを追加する場合、相対パスは正しく解決されません。URLベースの配布の場合は、GitHub、npm、またはGit URLソースを代わりに使用してください。詳細については、[トラブルシューティング](#plugins-with-relative-paths-fail-in-url-based-marketplaces)を参照してください。
</Note>

### GitHubリポジトリ

```json  theme={null}
{
  "name": "github-plugin",
  "source": {
    "source": "github",
    "repo": "owner/plugin-repo"
  }
}
```

### Gitリポジトリ

```json  theme={null}
{
  "name": "git-plugin",
  "source": {
    "source": "url",
    "url": "https://gitlab.com/team/plugin.git"
  }
}
```

### 高度なプラグインエントリ

この例は、コマンド、エージェント、フック、MCPサーバーのカスタムパスを含む、多くのオプションフィールドを使用するプラグインエントリを示しています：

```json  theme={null}
{
  "name": "enterprise-tools",
  "source": {
    "source": "github",
    "repo": "company/enterprise-plugin"
  },
  "description": "Enterprise workflow automation tools",
  "version": "2.1.0",
  "author": {
    "name": "Enterprise Team",
    "email": "enterprise@example.com"
  },
  "homepage": "https://docs.example.com/plugins/enterprise-tools",
  "repository": "https://github.com/company/enterprise-plugin",
  "license": "MIT",
  "keywords": ["enterprise", "workflow", "automation"],
  "category": "productivity",
  "commands": [
    "./commands/core/",
    "./commands/enterprise/",
    "./commands/experimental/preview.md"
  ],
  "agents": ["./agents/security-reviewer.md", "./agents/compliance-checker.md"],
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/scripts/validate.sh"
          }
        ]
      }
    ]
  },
  "mcpServers": {
    "enterprise-db": {
      "command": "${CLAUDE_PLUGIN_ROOT}/servers/db-server",
      "args": ["--config", "${CLAUDE_PLUGIN_ROOT}/config.json"]
    }
  },
  "strict": false
}
```

注意すべき重要な点：

* **`commands`と`agents`**：複数のディレクトリまたは個別のファイルを指定できます。パスはプラグインルートに相対的です。
* **`${CLAUDE_PLUGIN_ROOT}`**：フックとMCPサーバー設定でこの変数を使用して、プラグインのインストールディレクトリ内のファイルを参照します。これは、プラグインがインストール時にキャッシュロケーションにコピーされるため必要です。
* **`strict: false`**：これが`false`に設定されているため、プラグインは独自の`plugin.json`を必要としません。マーケットプレイスエントリがすべてを定義します。

## マーケットプレイスのホストと配布

### GitHubでホスト（推奨）

GitHubは最も簡単な配布方法を提供します：

1. **リポジトリの作成**：マーケットプレイス用の新しいリポジトリを設定します
2. **マーケットプレイスファイルの追加**：プラグイン定義を含む`.claude-plugin/marketplace.json`を作成します
3. **チームと共有**：ユーザーは`/plugin marketplace add owner/repo`でマーケットプレイスを追加します

**メリット**：組み込みのバージョン管理、問題追跡、チームコラボレーション機能。

### 他のGitサービスでホスト

GitLab、Bitbucket、自己ホスト型サーバーなど、任意のGitホスティングサービスが機能します。ユーザーは完全なリポジトリURLで追加します：

```shell  theme={null}
/plugin marketplace add https://gitlab.com/company/plugins.git
```

### プライベートリポジトリ

Claude Codeはプライベートリポジトリからプラグインをインストールすることをサポートしています。環境で適切な認証トークンを設定すると、Claude Codeは認証が必要な場合にそれを使用します。

| プロバイダー    | 環境変数                        | 注記                          |
| :-------- | :-------------------------- | :-------------------------- |
| GitHub    | `GITHUB_TOKEN`または`GH_TOKEN` | 個人アクセストークンまたはGitHub Appトークン |
| GitLab    | `GITLAB_TOKEN`または`GL_TOKEN` | 個人アクセストークンまたはプロジェクトトークン     |
| Bitbucket | `BITBUCKET_TOKEN`           | アプリパスワードまたはリポジトリアクセストークン    |

シェル設定（例：`.bashrc`、`.zshrc`）でトークンを設定するか、Claude Codeを実行するときに渡します：

```bash  theme={null}
export GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx
```

認証トークンは、リポジトリが認証を必要とする場合にのみ使用されます。公開リポジトリは、環境に存在するトークンがあっても、トークンなしで機能します。

<Note>
  CI/CD環境の場合、トークンをシークレット環境変数として設定します。GitHub Actionsは同じ組織内のリポジトリに対して自動的に`GITHUB_TOKEN`を提供します。
</Note>

### 配布前にローカルでテスト

共有する前にマーケットプレイスをローカルでテストします：

```shell  theme={null}
/plugin marketplace add ./my-local-marketplace
/plugin install test-plugin@my-local-marketplace
```

追加コマンドの完全な範囲（GitHub、Git URL、ローカルパス、リモートURL）については、[マーケットプレイスの追加](/ja/discover-plugins#add-marketplaces)を参照してください。

### チームのマーケットプレイスを必須にする

プロジェクトフォルダを信頼するときにチームメンバーが自動的にマーケットプレイスをインストールするよう促されるようにリポジトリを設定できます。マーケットプレイスを`.claude/settings.json`に追加します：

```json  theme={null}
{
  "extraKnownMarketplaces": {
    "company-tools": {
      "source": {
        "source": "github",
        "repo": "your-org/claude-plugins"
      }
    }
  }
}
```

デフォルトで有効にするプラグインを指定することもできます：

```json  theme={null}
{
  "enabledPlugins": {
    "code-formatter@company-tools": true,
    "deployment-tools@company-tools": true
  }
}
```

完全な設定オプションについては、[プラグイン設定](/ja/settings#plugin-settings)を参照してください。

### マネージドマーケットプレイスの制限

プラグインソースに対する厳密な制御を必要とする組織の場合、管理者は管理設定の[`strictKnownMarketplaces`](/ja/settings#strictknownmarketplaces)設定を使用して、ユーザーが追加できるプラグインマーケットプレイスを制限できます。

`strictKnownMarketplaces`が管理設定で設定されている場合、制限動作は値に依存します：

| 値          | 動作                                    |
| ---------- | ------------------------------------- |
| 未定義（デフォルト） | 制限なし。ユーザーは任意のマーケットプレイスを追加できます         |
| 空の配列`[]`   | 完全なロックダウン。ユーザーは新しいマーケットプレイスを追加できません   |
| ソースのリスト    | ユーザーはアローリストと正確に一致するマーケットプレイスのみを追加できます |

#### 一般的な設定

すべてのマーケットプレイス追加を無効にする：

```json  theme={null}
{
  "strictKnownMarketplaces": []
}
```

特定のマーケットプレイスのみを許可する：

```json  theme={null}
{
  "strictKnownMarketplaces": [
    {
      "source": "github",
      "repo": "acme-corp/approved-plugins"
    },
    {
      "source": "github",
      "repo": "acme-corp/security-tools",
      "ref": "v2.0"
    },
    {
      "source": "url",
      "url": "https://plugins.example.com/marketplace.json"
    }
  ]
}
```

#### 制限の仕組み

制限はプラグインインストールプロセスの早い段階で検証され、ネットワークリクエストやファイルシステム操作が発生する前に行われます。これにより、不正なマーケットプレイスアクセス試行を防ぎます。

アローリストは完全一致を使用します。マーケットプレイスが許可されるには、指定されたすべてのフィールドが正確に一致する必要があります：

* GitHubソースの場合：`repo`は必須であり、アローリストで指定されている場合は`ref`または`path`も一致する必要があります
* URLソースの場合：完全なURLが正確に一致する必要があります

`strictKnownMarketplaces`は[管理設定](/ja/settings#settings-files)で設定されるため、個々のユーザーとプロジェクト設定はこれらの制限をオーバーライドできません。

サポートされているすべてのソースタイプと`extraKnownMarketplaces`との比較を含む完全な設定詳細については、[strictKnownMarketplacesリファレンス](/ja/settings#strictknownmarketplaces)を参照してください。

## 検証とテスト

共有する前にマーケットプレイスをテストします。

マーケットプレイスJSON構文を検証します：

```bash  theme={null}
claude plugin validate .
```

またはClaude Code内から：

```shell  theme={null}
/plugin validate .
```

テスト用にマーケットプレイスを追加します：

```shell  theme={null}
/plugin marketplace add ./path/to/marketplace
```

テストプラグインをインストールしてすべてが機能することを確認します：

```shell  theme={null}
/plugin install test-plugin@marketplace-name
```

完全なプラグインテストワークフローについては、[プラグインをローカルでテスト](/ja/plugins#test-your-plugins-locally)を参照してください。技術的なトラブルシューティングについては、[プラグインリファレンス](/ja/plugins-reference)を参照してください。

## トラブルシューティング

### マーケットプレイスが読み込まれない

**症状**：マーケットプレイスを追加できない、またはそこからプラグインが見えない

**解決策**：

* マーケットプレイスURLがアクセス可能であることを確認します
* 指定されたパスに`.claude-plugin/marketplace.json`が存在することを確認します
* `claude plugin validate`または`/plugin validate`を使用してJSON構文が有効であることを確認します
* プライベートリポジトリの場合、アクセス権限があることを確認します

### マーケットプレイス検証エラー

マーケットプレイスディレクトリから`claude plugin validate .`または`/plugin validate .`を実行して、問題をチェックします。一般的なエラー：

| エラー                                               | 原因                    | 解決策                                               |
| :------------------------------------------------ | :-------------------- | :------------------------------------------------ |
| `File not found: .claude-plugin/marketplace.json` | マニフェストが見つかりません        | 必須フィールドを含む`.claude-plugin/marketplace.json`を作成します |
| `Invalid JSON syntax: Unexpected token...`        | JSON構文エラー             | 不足しているコンマ、余分なコンマ、または引用符なしの文字列をチェックします             |
| `Duplicate plugin name "x" found in marketplace`  | 2つのプラグインが同じ名前を共有しています | 各プラグインに一意の`name`値を付与します                           |
| `plugins[0].source: Path traversal not allowed`   | ソースパスに`..`が含まれています    | マーケットプレイスルートに相対的なパスを使用し、`..`を含めません                |

**警告**（ブロッキングなし）：

* `Marketplace has no plugins defined`：`plugins`配列に少なくとも1つのプラグインを追加します
* `No marketplace description provided`：ユーザーがマーケットプレイスを理解するのに役立つように`metadata.description`を追加します
* `Plugin "x" uses npm source which is not yet fully implemented`：代わりに`github`またはローカルパスソースを使用します

### プラグインインストール失敗

**症状**：マーケットプレイスは表示されるがプラグインインストールが失敗する

**解決策**：

* プラグインソースURLがアクセス可能であることを確認します
* プラグインディレクトリに必須ファイルが含まれていることを確認します
* GitHubソースの場合、リポジトリが公開されているか、アクセス権限があることを確認します
* プラグインソースを手動でクローン/ダウンロードしてテストします

### プライベートリポジトリ認証が失敗する

**症状**：トークンが設定されていても、プライベートリポジトリからプラグインをインストールするときに認証エラーが発生する

**解決策**：

* トークンが現在のシェルセッションで設定されていることを確認します：`echo $GITHUB_TOKEN`
* トークンに必要な権限（リポジトリへの読み取りアクセス）があることを確認します
* GitHubの場合、トークンがプライベートリポジトリの`repo`スコープを持つことを確認します
* GitLabの場合、トークンが少なくとも`read_repository`スコープを持つことを確認します
* トークンが期限切れになっていないことを確認します
* 複数のGitプロバイダーを使用している場合、正しいプロバイダーのトークンを設定していることを確認します

### 相対パスを持つプラグインがURLベースのマーケットプレイスで失敗する

**症状**：URL（`https://example.com/marketplace.json`など）経由でマーケットプレイスを追加しましたが、`"./plugins/my-plugin"`のような相対パスソースを持つプラグインが「パスが見つかりません」エラーで失敗します。

**原因**：URLベースのマーケットプレイスは`marketplace.json`ファイル自体のみをダウンロードします。サーバーからプラグインファイルをダウンロードしません。マーケットプレイスエントリの相対パスは、ダウンロードされなかったリモートサーバー上のファイルを参照します。

**解決策**：

* **外部ソースを使用**：プラグインエントリを相対パスの代わりにGitHub、npm、またはGit URLソースを使用するように変更します：
  ```json  theme={null}
  { "name": "my-plugin", "source": { "source": "github", "repo": "owner/repo" } }
  ```
* **Gitベースのマーケットプレイスを使用**：Gitリポジトリでマーケットプレイスをホストし、Git URLで追加します。Gitベースのマーケットプレイスはリポジトリ全体をクローンするため、相対パスが正しく機能します。

### インストール後にファイルが見つからない

**症状**：プラグインはインストールされますが、ファイルへの参照が失敗します。特にプラグインディレクトリの外のファイル

**原因**：プラグインはインプレイスで使用されるのではなく、キャッシュディレクトリにコピーされます。プラグインディレクトリの外のファイルを参照するパス（`../shared-utils`など）は、それらのファイルがコピーされないため機能しません。

**解決策**：シンボリックリンクとディレクトリ再構成を含むワークアラウンドについては、[プラグインキャッシングとファイル解決](/ja/plugins-reference#plugin-caching-and-file-resolution)を参照してください。

追加のデバッグツールと一般的な問題については、[デバッグと開発ツール](/ja/plugins-reference#debugging-and-development-tools)を参照してください。

## 関連項目

* [既成プラグインの検出とインストール](/ja/discover-plugins) - 既存のマーケットプレイスからプラグインをインストール
* [プラグイン](/ja/plugins) - 独自のプラグインの作成
* [プラグインリファレンス](/ja/plugins-reference) - 完全な技術仕様とスキーマ
* [プラグイン設定](/ja/settings#plugin-settings) - プラグイン設定オプション
* [strictKnownMarketplacesリファレンス](/ja/settings#strictknownmarketplaces) - マネージドマーケットプレイス制限

