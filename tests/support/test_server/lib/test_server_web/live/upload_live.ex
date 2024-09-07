defmodule TestServerWeb.SimpleLiveUpload do
  use TestServerWeb, :live_view
  use TestServerNative, :live_view

  @impl Phoenix.LiveView
  def mount(_params, _session, socket) do
    {:ok,
      socket
      |> assign(:uploaded_files, [])
      |> allow_upload(:avatar, accept: ~w(.png), max_entries: 2)}
  end

  @impl true
  def render(assigns) do
    ~H"""
    <.link href={~p"/stream"}>Streaming Page</.link>
    <p> THIS IS AN UPLOAD FORM </p>
    <form id="upload-form" phx-submit="save" phx-change="validate">

      <.live_file_input upload={@uploads.avatar} />

      <button type="submit">Upload</button>
    </form>
    """
  end

  @impl true
  def handle_event("validate", _params, socket) do
    {:noreply, socket}
  end

  @impl true
  def handle_event("cancel-upload", %{"ref" => ref}, socket) do
    {:noreply, cancel_upload(socket, :avatar, ref)}
  end

  @impl true
  def handle_event("save", _params, socket) do
    uploaded_files =
      consume_uploaded_entries(socket, :avatar, fn %{path: path}, _entry ->
        dest = Path.join([:code.priv_dir(:test_server), "static", "uploads", Path.basename(path)])
        dest = "#{dest}.png"
        IO.puts ("HANDLING SAVE EVENT: #{path} - #{dest}")
        File.cp!(path, dest)
        {:ok, ~p"/uploads/#{Path.basename(dest)}"}
      end)

    {:noreply, update(socket, :uploaded_files, &(&1 ++ uploaded_files))}
  end

  defp error_to_string(:too_large), do: "Too large"
  defp error_to_string(:too_many_files), do: "You have selected too many files"
  defp error_to_string(:not_accepted), do: "You have selected an unacceptable file type"
end

defmodule TestServerWeb.SimpleLiveUpload.SwiftUI do
  use TestServerNative, [:render_component, format: :swiftui]

  def render(assigns, _interface) do
    ~LVN"""
    <UploadForm>
      <Phoenix.Component.live_file_input upload={@uploads.avatar} />
    </UploadForm>
    """
  end
end

defmodule TestServerWeb.SimpleLiveUpload.Jetpack do
  use TestServerNative, [:render_component, format: :jetpack]

  def render(assigns, _) do
    ~LVN"""
    <Box size="fill" background="system-blue">
      <Text align="Center">Upload from Jetpack</Text>
      <Phoenix.Component.live_file_input upload={@uploads.avatar} />
    </Box>
    """
  end
end
