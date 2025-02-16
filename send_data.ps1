# Path to the raw PCM file
$pcmFilePath = "C:\Users\Leander\Music\3. Black or White.raw"

# Read the PCM file as binary data
$pcmData = [System.IO.File]::ReadAllBytes($pcmFilePath)

# Define the URL of the server (make sure your server is running)
$url = "http://192.168.178.82:80/upload"

# Create HttpClient
$client = New-Object System.Net.Http.HttpClient

# Split the PCM data into 1MB chunks (1MB = 1024 * 1024 bytes)
$chunkSize = 1KB
$numberOfChunks = [math]::Ceiling($pcmData.Length / $chunkSize)

for ($i = 0; $i -lt $numberOfChunks; $i++) {
    # Get the current chunk
    $start = $i * $chunkSize
    $end = [math]::Min(($start + $chunkSize), $pcmData.Length)
    $chunk = $pcmData[$start..($end - 1)]

    # Create ByteArrayContent for the chunk
    $byteContent = [System.Net.Http.ByteArrayContent]::new($chunk)

    Write-Host "Sending chunk $($i + 1) of $numberOfChunks, size: $($chunk.Length) bytes"

    # Set the content type to "application/octet-stream"
    $byteContent.Headers.Add("Content-Type", "application/octet-stream")

    # Send the POST request with the chunked PCM data
    $response = $client.PostAsync($url, $byteContent).Result

    # Print the response from the server
    # $response.Content.ReadAsStringAsync().Result
}
