defmodule TestServerWeb.SimpleLiveUpload do
  use TestServerWeb, :live_view
  use TestServerNative, :live_view

  @impl Phoenix.LiveView
  def mount(_params, _session, socket) do
    {:ok,
     socket
     |> assign(:uploaded_files, [])
     |> allow_upload(:avatar, accept: ~w(.png), max_entries: 2)
     |> allow_upload(:sample_text, accept: ~w(.txt))}
  end

  @impl true
  def render(assigns) do
    ~H"""
    <.link href={~p"/stream"}>Streaming Page</.link>
    <p> THIS IS AN UPLOAD FORM </p>
    <form id="upload-form" phx-submit="save" phx-change="validate">

      <.live_file_input upload={@uploads.avatar} />
      <.live_file_input upload={@uploads.sample_text} />

      <button type="submit">Upload</button>
    </form>
    """
  end

  @impl true
  def handle_event("validate", params, socket) do
    IO.puts("VALIDATION REQUEST: #{inspect(params)}")
    {:noreply, socket}
  end

  @impl true
  def handle_event("cancel-upload", %{"ref" => ref}, socket) do
    {:noreply, cancel_upload(socket, :avatar, ref)}
  end

  @impl true
  def handle_event("save", _, socket) do
    uploaded_files =
      consume_uploaded_entries(socket, :avatar, fn %{path: path}, _ ->
        dest = Path.join([:code.priv_dir(:test_server), "static", "uploads", Path.basename(path)])
        dest = "#{dest}.png"

        IO.puts("HANDLING SAVE EVENT: #{path} - #{dest}")
        File.cp!(path, dest)
        {:ok, "/uploads/#{Path.basename(dest)}"}
      end)

    # This is not idiomatic but I want to cut down on boiler plate
    # we need to add a way to inject upload params to the client
    _ =
      uploaded_files ++
        consume_uploaded_entries(socket, :sample_text, fn %{path: path}, _ ->
          dest =
            Path.join([:code.priv_dir(:test_server), "static", "uploads", Path.basename(path)])

          dest = "#{dest}.txt"

          IO.puts("HANDLING SAVE EVENT: #{path} - #{dest}")
          File.cp!(path, dest)
          {:ok, "/uploads/#{Path.basename(dest)}"}
        end)

    {:noreply, update(socket, :uploaded_files, &(&1 ++ uploaded_files))}
  end
end

defmodule TestServerWeb.SimpleLiveUpload.SwiftUI do
  use TestServerNative, [:render_component, format: :swiftui]

  def render(assigns, _interface) do
    ~LVN"""
    <UploadForm>
      <Phoenix.Component.live_file_input upload={@uploads.avatar} />
      <Phoenix.Component.live_file_input upload={@uploads.sample_text} />
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
      <Phoenix.Component.live_file_input upload={@uploads.sample_text} />
    </Box>
    """
  end
end
