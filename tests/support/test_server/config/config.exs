# This file is responsible for configuring your application
# and its dependencies with the aid of the Config module.
#
# This configuration file is loaded before any dependency and
# is restricted to this project.

# General application configuration
import Config

config :test_server,
  generators: [timestamp_type: :utc_datetime]

# Configures the endpoint
config :test_server, TestServerWeb.Endpoint,
  url: [host: "localhost"],
  adapter: Phoenix.Endpoint.Cowboy2Adapter,
  render_errors: [
    formats: [html: TestServerWeb.ErrorHTML, json: TestServerWeb.ErrorJSON],
    layout: false
  ],
  pubsub_server: TestServer.PubSub,
  live_view: [signing_salt: "B40/4H2u"]

# Configures the mailer
#
# By default it uses the "Local" adapter which stores the emails
# locally. You can see the emails in your browser, at "/dev/mailbox".
#
# For production it's recommended to configure a different adapter
# at the `config/runtime.exs`.
config :test_server, TestServer.Mailer, adapter: Swoosh.Adapters.Local

config :live_view_native, plugins: [
  LiveViewNative.SwiftUI,
  LiveViewNative.Jetpack,
]

config :mime, :types, %{
  "text/swiftui" => ["swiftui"],
  "text/jetpack" => ["jetpack"],
  "text/styles" => ["styles"]
}

# LVN - Required, you must configure LiveView Native Stylesheets
# on where class names shoudl be extracted from
config :live_view_native_stylesheet,
  content: [
    swiftui: [
      "lib/**/*swiftui*"
    ],
    jetpack: [
      "lib/**/*jetpack*"
    ]
  ],
  output: "priv/static/assets"

config :live_view_native_stylesheet,
  parsers: [
    swiftui: LiveViewNative.SwiftUI.RulesParser
  ]

# LVN - Required, you must configure Phoenix to know how
# to encode for the swiftui format
config :phoenix_template, :format_encoders, [
  swiftui: Phoenix.HTML.Engine,
  jetpack: Phoenix.HTML.Engine
]

# LVN - Required, you must configure Phoenix so it knows
# how to compile LVN's neex templates
config :phoenix, :template_engines, neex: LiveViewNative.Engine

# Configure esbuild (the version is required)
config :esbuild,
  version: "0.17.11",
  default: [
    args:
      ~w(js/app.js --bundle --target=es2017 --outdir=../priv/static/assets --external:/fonts/* --external:/images/*),
    cd: Path.expand("../assets", __DIR__),
    env: %{"NODE_PATH" => Path.expand("../deps", __DIR__)}
  ]

# Configure tailwind (the version is required)
config :tailwind,
  version: "3.3.2",
  default: [
    args: ~w(
      --config=tailwind.config.js
      --input=css/app.css
      --output=../priv/static/assets/app.css
    ),
    cd: Path.expand("../assets", __DIR__)
  ]

# Configures Elixir's Logger
config :logger, :console,
  format: "$time $metadata[$level] $message\n",
  metadata: [:request_id]

# Use Jason for JSON parsing in Phoenix
config :phoenix, :json_library, Jason

# Import environment specific config. This must remain at the bottom
# of this file so it overrides the configuration defined above.
import_config "#{config_env()}.exs"
