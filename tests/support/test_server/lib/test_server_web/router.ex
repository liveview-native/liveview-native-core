defmodule TestServerWeb.Router do
  use TestServerWeb, :router

  pipeline :browser do
    plug(:accepts, ["html", "swiftui", "jetpack"])
    plug(:fetch_session)
    plug(:fetch_live_flash)

    plug(:put_root_layout,
      html: {TestServerWeb.Layouts, :root},
      swiftui: {TestServerWeb.Layouts.SwiftUI, :root},
      jetpack: {TestServerWeb.Layouts.Jetpack, :root}
    )

    plug(:protect_from_forgery)
    plug(:put_secure_browser_headers)
  end

  pipeline :api do
    plug(:accepts, ["json"])
  end

  scope "/redirect", TestServerWeb do
    pipe_through([:browser, :redirect])

    live_session :redirect,
      on_mount: [{TestServerWeb.Redirect, :redirect}] do
      live("/redirect/finale", UserRegistrationLive, :new)
    end
  end

  scope "/", TestServerWeb do
    pipe_through(:browser)

    get("/", PageController, :home)

    live("/redirect_from", RedirectLive)
    live("/redirect_to", RedirectToLive)
    live("/thermostat", ThermostatLive)
    live("/hello", HelloLive)
    live("/nav/:dynamic", NavLive)
    live("/upload", SimpleLiveUpload)
    live("/stream", SimpleLiveStream)
    live("/push_navigate", RedirectExampleLive)
  end

  # Other scopes may use custom stacks.
  # scope "/api", TestServerWeb do
  #   pipe_through :api
  # end

  # Enable LiveDashboard and Swoosh mailbox preview in development
  if Application.compile_env(:test_server, :dev_routes) do
    # If you want to use the LiveDashboard in production, you should put
    # it behind authentication and allow only admins to access it.
    # If your application does not have an admins-only section yet,
    # you can use Plug.BasicAuth to set up some basic authentication
    # as long as you are also using SSL (which you should anyway).
    import Phoenix.LiveDashboard.Router

    scope "/dev" do
      pipe_through(:browser)

      live_dashboard("/dashboard", metrics: TestServerWeb.Telemetry)
      forward("/mailbox", Plug.Swoosh.MailboxPreview)
    end
  end
end
