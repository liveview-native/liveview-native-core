defmodule TestServerWeb.CookieLive do
  use TestServerWeb, :live_view
  use TestServerNative, :live_view

  def mount(_params, _session, socket) do
    conn = Phoenix.LiveView.get_connect_info(socket, :peer_data)
    Plug.Conn.put_resp_header(conn, "my-custom-header", "header-value")

    socket =
      case socket.assigns.live_action do
        :set ->
          socket

        :check ->
          if Map.has_key?(conn.cookies, "persistent_key") do
            socket
          else
            raise "Missing persistent_key cookie"
          end
      end

    {:ok, socket}
  rescue
    e in RuntimeError -> {:error, e.message}
  end

  def render(assigns) do
    ~H"""
    <div>
      Check Networking Header Info
    </div>
    """
  end

  defmodule TestServerWeb.CookieLive.SwiftUI do
    use TestServerNative, [:render_component, format: :swiftui]

    def render(assigns, _interface) do
      ~LVN"""
      <VStack>
        <Text>
          Check Networking Header Info
        </Text>
      </VStack>
      """
    end
  end
end
