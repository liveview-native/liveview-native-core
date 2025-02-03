defmodule TestServerWeb.RedirectExampleLive do
  use TestServerWeb, :live_view

  def mount(params, _session, socket) do
    case Map.get(params, "redirect_type") do
      "patch" ->
        {:ok, socket}

      "push_redirect" ->
        {:ok, push_redirect(socket, to: "/redirect_to")}

      "redirect" ->
        {:ok, redirect(socket, to: "/redirect_to")}

      "live_redirect" ->
        {:ok, push_navigate(socket, to: "/redirect_to")}

      _ ->
        {:error, socket}
    end
  end

  def handle_event("patchme", _params, socket) do
    {:noreply, push_patch(socket, to: "/push_navigate?patched=value")}
  end

  def handle_params(params, _uri, socket) do
    {:noreply, assign(socket, params: params)}
  end

  def render(assigns) do
    ~H"""
    <p>
      you should have been redirected
    </p>
    """
  end
end

defmodule TestServerWeb.RedirectExampleLive.Jetpack do
  use TestServerNative, [:render_component, format: :jetpack]

  def render(assigns, _) do
    ~LVN"""
    <Box size="fill" background="system-blue">
      <Text align="Center">
        Should Have Been Redirected
      </Text>
    </Box>
    """
  end
end

defmodule TestServerWeb.RedirectExampleLive.SwiftUI do
  use TestServerNative, [:render_component, format: :swiftui]

  def render(assigns, _interface) do
    ~LVN"""
    <VStack>
      <Text>
        Should Have Been Redirected
      </Text>
    </VStack>
    """
  end
end
